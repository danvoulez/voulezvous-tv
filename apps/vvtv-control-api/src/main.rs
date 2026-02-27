use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::{
    Json, Router,
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::sync::RwLock;
use tracing::info;
use vvtv_store::{AlertStateRecord, ReportData, StateStore};
use vvtv_types::{DailyReport, PipelineMetrics, WeeklyReport};

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
struct ApiState {
    emergency_mode: Arc<RwLock<bool>>,
    buffer_minutes: Arc<RwLock<i64>>,
    control_token: String,
    control_secret: String,
    state_db_path: String,
    webhook_url: Option<String>,
    alert_cooldown_secs: i64,
    qa_min_threshold: f32,
    fallback_growth_delta: f32,
    fallback_abs_threshold: f32,
    discovery_fail_threshold: usize,
}

#[derive(Deserialize)]
struct DailyQuery {
    date: String,
}

#[derive(Deserialize)]
struct WeeklyQuery {
    week: String,
}

#[derive(Debug, Clone, Serialize)]
struct AlertItem {
    code: String,
    severity: String,
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter("info")
        .init();
    validate_startup_config()?;

    let state = ApiState {
        emergency_mode: Arc::new(RwLock::new(false)),
        buffer_minutes: Arc::new(RwLock::new(60)),
        control_token: std::env::var("VVTV_CONTROL_TOKEN")
            .unwrap_or_else(|_| "dev-token".to_string()),
        control_secret: std::env::var("VVTV_CONTROL_SECRET")
            .unwrap_or_else(|_| "dev-secret".to_string()),
        state_db_path: std::env::var("VVTV_STATE_DB")
            .unwrap_or_else(|_| "runtime/state/vvtv.db".to_string()),
        webhook_url: std::env::var("VVTV_ALERT_WEBHOOK_URL").ok(),
        alert_cooldown_secs: std::env::var("VVTV_ALERT_COOLDOWN_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(900),
        qa_min_threshold: std::env::var("VVTV_ALERT_QA_MIN")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.85),
        fallback_growth_delta: std::env::var("VVTV_ALERT_FALLBACK_GROWTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.15),
        fallback_abs_threshold: std::env::var("VVTV_ALERT_FALLBACK_ABS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.30),
        discovery_fail_threshold: std::env::var("VVTV_ALERT_DISCOVERY_FAIL_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
    };

    if state.webhook_url.is_some() {
        let dispatch_state = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(err) = dispatch_alert_notifications(&dispatch_state).await {
                    info!(error = %err, "alert-dispatch-failed");
                }
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        });
    }

    let control_routes = Router::new()
        .route("/reload-owner-card", post(reload_owner_card))
        .route("/emergency-mode", post(toggle_emergency))
        .route("/curator-mode", post(set_curator_mode))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_control_auth,
        ));

    let app = Router::new()
        .route("/v1/status", get(status))
        .route("/v1/reports/daily", get(daily_report))
        .route("/v1/reports/weekly", get(weekly_report))
        .route("/v1/alerts", get(alerts))
        .route("/metrics", get(prometheus_metrics))
        .nest("/v1/control", control_routes)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:7070").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn status(State(state): State<ApiState>) -> impl IntoResponse {
    let emergency = *state.emergency_mode.read().await;
    let buffer = *state.buffer_minutes.read().await;
    Json(serde_json::json!({
        "state": if emergency { "EMERGENCY" } else { "RUNNING" },
        "buffer_minutes": buffer,
        "timestamp": Utc::now().to_rfc3339(),
    }))
}

async fn daily_report(
    State(state): State<ApiState>,
    Query(query): Query<DailyQuery>,
) -> impl IntoResponse {
    let date = match NaiveDate::parse_from_str(&query.date, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid date format, use YYYY-MM-DD" })),
            )
                .into_response();
        }
    };

    let start = date.and_hms_opt(0, 0, 0).expect("valid day").and_utc();
    let end = (date + Duration::days(1))
        .and_hms_opt(0, 0, 0)
        .expect("valid day")
        .and_utc();

    match build_report_from_range(
        &state.state_db_path,
        start,
        end,
        Some(query.date.clone()),
        None,
    ) {
        Ok(report) => Json(report).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed building daily report: {err}") })),
        )
            .into_response(),
    }
}

async fn weekly_report(
    State(state): State<ApiState>,
    Query(query): Query<WeeklyQuery>,
) -> impl IntoResponse {
    let (iso_year, iso_week) = match parse_iso_week(&query.week) {
        Some(v) => v,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid week format, use YYYY-Www" })),
            )
                .into_response();
        }
    };
    let start_date = match NaiveDate::from_isoywd_opt(iso_year, iso_week, Weekday::Mon) {
        Some(d) => d,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid iso week" })),
            )
                .into_response();
        }
    };
    let start = start_date
        .and_hms_opt(0, 0, 0)
        .expect("valid day")
        .and_utc();
    let end = (start_date + Duration::days(7))
        .and_hms_opt(0, 0, 0)
        .expect("valid day")
        .and_utc();

    match build_report_from_range(
        &state.state_db_path,
        start,
        end,
        None,
        Some(query.week.clone()),
    ) {
        Ok(report) => Json(report).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed building weekly report: {err}") })),
        )
            .into_response(),
    }
}

async fn alerts(State(state): State<ApiState>) -> impl IntoResponse {
    let alerts = evaluate_alerts(&state).unwrap_or_default();
    Json(serde_json::json!({
        "count": alerts.len(),
        "alerts": alerts,
    }))
}

async fn prometheus_metrics(State(state): State<ApiState>) -> impl IntoResponse {
    let metrics = load_live_metrics(&state).unwrap_or_else(|_| sample_metrics());
    let body = format!(
        "# HELP vvtv_buffer_minutes Buffer minutes ready for stream\n\
# TYPE vvtv_buffer_minutes gauge\n\
vvtv_buffer_minutes {}\n\
# HELP vvtv_plans_created Plans discovered/planned\n\
# TYPE vvtv_plans_created gauge\n\
vvtv_plans_created {}\n\
# HELP vvtv_plans_committed Plans committed at T-4h\n\
# TYPE vvtv_plans_committed gauge\n\
vvtv_plans_committed {}\n\
# HELP vvtv_qa_pass_rate QA pass ratio\n\
# TYPE vvtv_qa_pass_rate gauge\n\
vvtv_qa_pass_rate {}\n\
# HELP vvtv_fallback_rate Fallback ratio\n\
# TYPE vvtv_fallback_rate gauge\n\
vvtv_fallback_rate {}\n\
# HELP vvtv_curator_actions Curator actions\n\
# TYPE vvtv_curator_actions gauge\n\
vvtv_curator_actions {}\n\
# HELP vvtv_stream_disruptions Stream disruptions\n\
# TYPE vvtv_stream_disruptions gauge\n\
vvtv_stream_disruptions {}\n",
        metrics.buffer_minutes,
        metrics.plans_created,
        metrics.plans_committed,
        metrics.qa_pass_rate,
        metrics.fallback_rate,
        metrics.curator_actions,
        metrics.stream_disruptions,
    );
    (StatusCode::OK, body)
}

async fn reload_owner_card() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true, "action": "reload-owner-card" }))
}

async fn toggle_emergency(State(state): State<ApiState>) -> impl IntoResponse {
    let mut guard = state.emergency_mode.write().await;
    *guard = !*guard;
    Json(serde_json::json!({ "ok": true, "emergency_mode": *guard }))
}

async fn set_curator_mode() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true, "mode": "automatic-with-guardrails" }))
}

async fn require_control_auth(
    State(state): State<ApiState>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let expected_auth = format!("Bearer {}", state.control_token);
    if auth_header != expected_auth {
        return (StatusCode::UNAUTHORIZED, "invalid token").into_response();
    }

    let ts = request
        .headers()
        .get("x-vvtv-ts")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let signature = request
        .headers()
        .get("x-vvtv-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if ts.is_empty() || signature.is_empty() {
        return (StatusCode::UNAUTHORIZED, "missing signature headers").into_response();
    }

    if !timestamp_fresh(ts) {
        return (StatusCode::UNAUTHORIZED, "stale timestamp").into_response();
    }

    let method = request.method().as_str();
    let path = request
        .extensions()
        .get::<axum::extract::OriginalUri>()
        .map_or_else(
            || request.uri().path().to_string(),
            |u| u.0.path().to_string(),
        );
    let expected = match sign(&state.control_secret, method, &path, ts, "") {
        Ok(s) => s,
        Err(_) => return (StatusCode::UNAUTHORIZED, "cannot sign request").into_response(),
    };

    if expected != signature {
        return (StatusCode::UNAUTHORIZED, "invalid signature").into_response();
    }

    next.run(request).await
}

fn timestamp_fresh(ts: &str) -> bool {
    let parsed = match ts.parse::<i64>() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let now = Utc::now().timestamp();
    (now - parsed).abs() <= 300
}

fn sign(secret: &str, method: &str, path: &str, ts: &str, body: &str) -> anyhow::Result<String> {
    let canonical = format!("{method}\n{path}\n{ts}\n{body}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
    mac.update(canonical.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(hex_string(&result))
}

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn load_live_metrics(state: &ApiState) -> anyhow::Result<PipelineMetrics> {
    let store = StateStore::open(&state.state_db_path)?;
    Ok(store.load_latest_metrics()?.unwrap_or_else(sample_metrics))
}

fn evaluate_alerts(state: &ApiState) -> anyhow::Result<Vec<AlertItem>> {
    let store = StateStore::open(&state.state_db_path)?;
    let latest = store.load_latest_metrics()?.unwrap_or_else(sample_metrics);
    let recent = store.load_recent_metrics(2)?;
    let audits_24h = store.load_recent_audits(24)?;

    let mut out = Vec::new();

    if latest.buffer_minutes < 20 {
        out.push(AlertItem {
            code: "BUFFER_CRITICAL".to_string(),
            severity: "critical".to_string(),
            message: format!("buffer_minutes={} is below 20", latest.buffer_minutes),
        });
    }

    if latest.qa_pass_rate < state.qa_min_threshold {
        out.push(AlertItem {
            code: "QA_PASS_RATE_LOW".to_string(),
            severity: "high".to_string(),
            message: format!(
                "qa_pass_rate={} is below threshold {}",
                latest.qa_pass_rate, state.qa_min_threshold
            ),
        });
    }

    if latest.fallback_rate > state.fallback_abs_threshold {
        out.push(AlertItem {
            code: "FALLBACK_RATE_HIGH".to_string(),
            severity: "high".to_string(),
            message: format!(
                "fallback_rate={} is above absolute threshold {}",
                latest.fallback_rate, state.fallback_abs_threshold
            ),
        });
    }

    if recent.len() >= 2 {
        let prev = &recent[1];
        if latest.fallback_rate - prev.fallback_rate > state.fallback_growth_delta {
            out.push(AlertItem {
                code: "FALLBACK_RATE_GROWING".to_string(),
                severity: "medium".to_string(),
                message: format!(
                    "fallback_rate grew from {} to {} (> delta {})",
                    prev.fallback_rate, latest.fallback_rate, state.fallback_growth_delta
                ),
            });
        }
    }

    let discovery_fail_count = audits_24h
        .iter()
        .filter(|a| a.reason_code.contains("DISCOVERY_FAILED_DOMAIN"))
        .count();
    if discovery_fail_count >= state.discovery_fail_threshold {
        out.push(AlertItem {
            code: "DISCOVERY_DOMAIN_FAILURE".to_string(),
            severity: "high".to_string(),
            message: format!(
                "discovery domain failures in last 24h: {} (threshold {})",
                discovery_fail_count, state.discovery_fail_threshold
            ),
        });
    }

    Ok(out)
}

async fn dispatch_alert_notifications(state: &ApiState) -> anyhow::Result<()> {
    let webhook_url = match &state.webhook_url {
        Some(v) => v,
        None => return Ok(()),
    };

    let alerts = evaluate_alerts(state)?;
    let actionable: Vec<AlertItem> = alerts
        .into_iter()
        .filter(|a| a.severity == "critical" || a.severity == "high")
        .collect();

    let mut store = StateStore::open(&state.state_db_path)?;
    let now = Utc::now();

    let existing = store.load_alert_states()?;
    let existing_map: HashMap<String, AlertStateRecord> = existing
        .iter()
        .cloned()
        .map(|s| (s.code.clone(), s))
        .collect();

    let mut active_now = HashSet::new();
    for alert in &actionable {
        active_now.insert(alert.code.clone());
        let prior = existing_map.get(&alert.code);
        let prior_active = prior.is_some_and(|s| s.active);
        let last_notified = prior
            .and_then(|s| s.last_notified_at.as_deref())
            .and_then(parse_rfc3339_utc);

        let should_notify = if !prior_active {
            true
        } else {
            last_notified
                .map(|ts| now - ts >= Duration::seconds(state.alert_cooldown_secs))
                .unwrap_or(true)
        };

        let mut next_notified = last_notified;
        if should_notify {
            send_webhook(
                webhook_url,
                serde_json::json!({
                    "event": "alert",
                    "code": alert.code,
                    "severity": alert.severity,
                    "message": alert.message,
                    "timestamp": now.to_rfc3339(),
                }),
            )
            .await?;
            next_notified = Some(now);
            info!(
                code = alert.code,
                severity = alert.severity,
                "alert-webhook-sent"
            );
        }

        store.upsert_alert_state(&alert.code, true, next_notified)?;
    }

    for prior in existing {
        if prior.active && !active_now.contains(&prior.code) {
            send_webhook(
                webhook_url,
                serde_json::json!({
                    "event": "alert_clear",
                    "code": prior.code,
                    "timestamp": now.to_rfc3339(),
                }),
            )
            .await?;
            store.upsert_alert_state(&prior.code, false, Some(now))?;
            info!(code = prior.code, "alert-clear-webhook-sent");
        }
    }

    Ok(())
}

async fn send_webhook(url: &str, payload: serde_json::Value) -> anyhow::Result<()> {
    reqwest::Client::new()
        .post(url)
        .json(&payload)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn parse_rfc3339_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn validate_startup_config() -> Result<()> {
    let env = std::env::var("VVTV_ENV").unwrap_or_else(|_| "dev".to_string());
    if env == "dev" {
        return Ok(());
    }
    let token = std::env::var("VVTV_CONTROL_TOKEN")
        .map_err(|_| anyhow!("VVTV_CONTROL_TOKEN is required in non-dev"))?;
    let secret = std::env::var("VVTV_CONTROL_SECRET")
        .map_err(|_| anyhow!("VVTV_CONTROL_SECRET is required in non-dev"))?;
    if token == "dev-token" || secret == "dev-secret" {
        return Err(anyhow!(
            "default dev secrets are not allowed when VVTV_ENV != dev"
        ));
    }
    Ok(())
}

fn sample_metrics() -> PipelineMetrics {
    PipelineMetrics {
        buffer_minutes: 60,
        plans_created: 42,
        plans_committed: 18,
        qa_pass_rate: 0.94,
        fallback_rate: 0.12,
        curator_actions: 3,
        stream_disruptions: 0,
    }
}

fn parse_iso_week(raw: &str) -> Option<(i32, u32)> {
    let (year, week) = raw.split_once("-W")?;
    let iso_year: i32 = year.parse().ok()?;
    let iso_week: u32 = week.parse().ok()?;
    if !(1..=53).contains(&iso_week) {
        return None;
    }
    Some((iso_year, iso_week))
}

fn build_report_from_range(
    db_path: &str,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
    date: Option<String>,
    week: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    let store = StateStore::open(db_path)?;
    let data = store.load_report_data_between(start, end)?;
    let metrics = aggregate_metrics(&data);
    let summary = summarize_report(&data, &metrics);

    if let Some(date) = date {
        let report = DailyReport {
            date,
            summary,
            metrics,
        };
        return Ok(serde_json::to_value(report)?);
    }

    let report = WeeklyReport {
        week: week.unwrap_or_else(|| format!("{}-W{:02}", start.year(), start.iso_week().week())),
        summary,
        metrics,
    };
    Ok(serde_json::to_value(report)?)
}

fn aggregate_metrics(data: &ReportData) -> PipelineMetrics {
    if data.metrics.is_empty() {
        let qa_pass = data
            .assets
            .iter()
            .filter(|a| a.qa_status == vvtv_types::QaStatus::Passed)
            .count();
        let qa_pass_rate = if data.assets.is_empty() {
            0.0
        } else {
            qa_pass as f32 / data.assets.len() as f32
        };
        let fallback_count = data
            .audits
            .iter()
            .filter(|a| a.reason_code.contains("FALLBACK"))
            .count();
        let fallback_rate = if data.plans.is_empty() {
            0.0
        } else {
            fallback_count as f32 / data.plans.len() as f32
        };
        let curator_actions = data
            .audits
            .iter()
            .filter(|a| a.module == "vvtv-curator")
            .count();
        return PipelineMetrics {
            buffer_minutes: 0,
            plans_created: data.plans.len(),
            plans_committed: data.assets.len(),
            qa_pass_rate,
            fallback_rate,
            curator_actions,
            stream_disruptions: 0,
        };
    }

    let mut buffer_sum = 0_i64;
    let mut plans_created_sum = 0_usize;
    let mut plans_committed_sum = 0_usize;
    let mut qa_pass_sum = 0.0_f32;
    let mut fallback_sum = 0.0_f32;
    let mut curator_sum = 0_usize;
    let mut disruptions_sum = 0_usize;

    for m in &data.metrics {
        buffer_sum += m.buffer_minutes;
        plans_created_sum += m.plans_created;
        plans_committed_sum += m.plans_committed;
        qa_pass_sum += m.qa_pass_rate;
        fallback_sum += m.fallback_rate;
        curator_sum += m.curator_actions;
        disruptions_sum += m.stream_disruptions;
    }

    let count = data.metrics.len() as i64;
    let fcount = data.metrics.len() as f32;
    PipelineMetrics {
        buffer_minutes: if count == 0 { 0 } else { buffer_sum / count },
        plans_created: plans_created_sum / data.metrics.len().max(1),
        plans_committed: plans_committed_sum / data.metrics.len().max(1),
        qa_pass_rate: if fcount == 0.0 {
            0.0
        } else {
            qa_pass_sum / fcount
        },
        fallback_rate: if fcount == 0.0 {
            0.0
        } else {
            fallback_sum / fcount
        },
        curator_actions: curator_sum / data.metrics.len().max(1),
        stream_disruptions: disruptions_sum / data.metrics.len().max(1),
    }
}

fn summarize_report(data: &ReportData, metrics: &PipelineMetrics) -> String {
    let mut domains: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for p in &data.plans {
        *domains.entry(p.source_domain.as_str()).or_default() += 1;
    }
    let top_domains = domains
        .iter()
        .map(|(d, c)| format!("{d}:{c}"))
        .take(3)
        .collect::<Vec<_>>()
        .join(", ");

    let qa_failed = data
        .assets
        .iter()
        .filter(|a| a.qa_status == vvtv_types::QaStatus::Rejected)
        .count();
    let fallback_events = data
        .audits
        .iter()
        .filter(|a| a.reason_code.contains("FALLBACK"))
        .count();
    let curator_actions = data
        .audits
        .iter()
        .filter(|a| a.module == "vvtv-curator")
        .count();

    format!(
        "plans={} assets={} qa_failed={} fallback_events={} curator_actions={} avg_buffer={}m domains=[{}] qa_pass_rate={:.2} fallback_rate={:.2}",
        data.plans.len(),
        data.assets.len(),
        qa_failed,
        fallback_events,
        curator_actions,
        metrics.buffer_minutes,
        top_domains,
        metrics.qa_pass_rate,
        metrics.fallback_rate
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_window_works() {
        let now = Utc::now().timestamp();
        assert!(timestamp_fresh(&now.to_string()));
        assert!(!timestamp_fresh(&(now - 1000).to_string()));
    }

    #[test]
    fn signature_stable() {
        let a = sign(
            "dev-secret",
            "POST",
            "/v1/control/emergency-mode",
            "1700000000",
            "",
        )
        .expect("sign");
        let b = sign(
            "dev-secret",
            "POST",
            "/v1/control/emergency-mode",
            "1700000000",
            "",
        )
        .expect("sign");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }
}
