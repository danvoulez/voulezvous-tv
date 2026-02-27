use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use parking_lot::Mutex;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::Sha256;
use vvtv_types::{DailyReport, PipelineMetrics, WeeklyReport};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub state: String,
    pub buffer_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub ok: bool,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    pub max_retries: u32,
    pub base_backoff_ms: u64,
    pub failure_threshold: u32,
    pub circuit_cooldown_secs: i64,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_backoff_ms: 250,
            failure_threshold: 3,
            circuit_cooldown_secs: 30,
        }
    }
}

#[derive(Debug, Default)]
struct CircuitState {
    consecutive_failures: u32,
    open_until: Option<DateTime<Utc>>,
}

pub struct ControlAgent {
    base_url: String,
    client: Client,
    token: Option<String>,
    signing_secret: Option<String>,
    resilience: ResilienceConfig,
    circuit: Arc<Mutex<CircuitState>>,
}

impl ControlAgent {
    #[must_use]
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
            token: None,
            signing_secret: None,
            resilience: ResilienceConfig::default(),
            circuit: Arc::new(Mutex::new(CircuitState::default())),
        }
    }

    #[must_use]
    pub fn with_auth(
        mut self,
        token: impl Into<String>,
        signing_secret: impl Into<String>,
    ) -> Self {
        self.token = Some(token.into());
        self.signing_secret = Some(signing_secret.into());
        self
    }

    #[must_use]
    pub fn with_resilience(mut self, resilience: ResilienceConfig) -> Self {
        self.resilience = resilience;
        self
    }

    pub async fn status(&self) -> Result<StatusResponse> {
        self.execute_json(Method::GET, "/v1/status", None, None)
            .await
    }

    pub async fn daily_report(&self, date: &str) -> Result<DailyReport> {
        self.execute_json(
            Method::GET,
            "/v1/reports/daily",
            Some(vec![("date", date)]),
            None,
        )
        .await
    }

    pub async fn weekly_report(&self, week: &str) -> Result<WeeklyReport> {
        self.execute_json(
            Method::GET,
            "/v1/reports/weekly",
            Some(vec![("week", week)]),
            None,
        )
        .await
    }

    pub async fn reload_owner_card(&self) -> Result<ControlResponse> {
        self.execute_json(
            Method::POST,
            "/v1/control/reload-owner-card",
            None,
            Some(""),
        )
        .await
    }

    pub async fn toggle_emergency_mode(&self) -> Result<ControlResponse> {
        self.execute_json(Method::POST, "/v1/control/emergency-mode", None, Some(""))
            .await
    }

    pub async fn set_curator_mode(&self) -> Result<ControlResponse> {
        self.execute_json(Method::POST, "/v1/control/curator-mode", None, Some(""))
            .await
    }

    pub async fn publish_status_snapshot(
        &self,
        metrics: &PipelineMetrics,
    ) -> Result<IngestResponse> {
        let body = serde_json::to_string(metrics)?;
        self.execute_json(Method::POST, "/v1/ingest/status", None, Some(body.as_str()))
            .await
    }

    pub async fn publish_daily_report(&self, report: &DailyReport) -> Result<IngestResponse> {
        let body = serde_json::to_string(report)?;
        self.execute_json(Method::POST, "/v1/ingest/daily", None, Some(body.as_str()))
            .await
    }

    pub async fn publish_weekly_report(&self, report: &WeeklyReport) -> Result<IngestResponse> {
        let body = serde_json::to_string(report)?;
        self.execute_json(Method::POST, "/v1/ingest/weekly", None, Some(body.as_str()))
            .await
    }

    async fn execute_json<T>(
        &self,
        method: Method,
        path: &str,
        query: Option<Vec<(&str, &str)>>,
        body: Option<&str>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        self.ensure_circuit_closed()?;

        let body_str = body.unwrap_or("");
        for attempt in 0..=self.resilience.max_retries {
            let url = format!("{}{}", self.base_url, path);
            let mut req = self.client.request(method.clone(), url);
            if let Some(query_pairs) = &query {
                req = req.query(query_pairs);
            }
            if !body_str.is_empty() {
                req = req.body(body_str.to_string());
            }
            if let (Some(token), Some(secret)) = (&self.token, &self.signing_secret) {
                let ts = Utc::now().timestamp().to_string();
                let sig = sign(secret, method.as_str(), path, &ts, body_str)?;
                req = req
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-vvtv-ts", ts)
                    .header("x-vvtv-signature", sig);
            }

            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        self.mark_success();
                        return Ok(resp.json::<T>().await?);
                    }

                    if is_retryable_status(status) && attempt < self.resilience.max_retries {
                        self.mark_transient_failure();
                        tokio::time::sleep(backoff_for(attempt, self.resilience.base_backoff_ms))
                            .await;
                        continue;
                    }

                    if is_retryable_status(status) {
                        self.mark_transient_failure();
                    }
                    return Err(anyhow!("control-agent http error: status={status}"));
                }
                Err(err) => {
                    let retryable = err.is_connect() || err.is_timeout() || err.is_request();
                    if retryable {
                        self.mark_transient_failure();
                        if attempt < self.resilience.max_retries {
                            tokio::time::sleep(backoff_for(
                                attempt,
                                self.resilience.base_backoff_ms,
                            ))
                            .await;
                            continue;
                        }
                    }
                    return Err(err.into());
                }
            }
        }

        Err(anyhow!("control-agent exhausted retries without response"))
    }

    fn ensure_circuit_closed(&self) -> Result<()> {
        let state = self.circuit.lock();
        if let Some(open_until) = state.open_until
            && Utc::now() < open_until
        {
            return Err(anyhow!(
                "control-agent circuit open until {}",
                open_until.to_rfc3339()
            ));
        }
        Ok(())
    }

    fn mark_success(&self) {
        let mut state = self.circuit.lock();
        state.consecutive_failures = 0;
        state.open_until = None;
    }

    fn mark_transient_failure(&self) {
        let mut state = self.circuit.lock();
        state.consecutive_failures += 1;
        if state.consecutive_failures >= self.resilience.failure_threshold {
            state.open_until =
                Some(Utc::now() + Duration::seconds(self.resilience.circuit_cooldown_secs));
            state.consecutive_failures = 0;
        }
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

fn backoff_for(attempt: u32, base_ms: u64) -> std::time::Duration {
    let factor = 1_u64 << attempt.min(6);
    std::time::Duration::from_millis(base_ms.saturating_mul(factor))
}

fn sign(secret: &str, method: &str, path: &str, ts: &str, body: &str) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_is_stable() {
        let a = sign(
            "secret",
            "POST",
            "/v1/control/emergency-mode",
            "1700000000",
            "",
        )
        .expect("sign");
        let b = sign(
            "secret",
            "POST",
            "/v1/control/emergency-mode",
            "1700000000",
            "",
        )
        .expect("sign");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn backoff_grows_exponentially() {
        let d1 = backoff_for(0, 100);
        let d2 = backoff_for(1, 100);
        let d3 = backoff_for(2, 100);
        assert_eq!(d1.as_millis(), 100);
        assert_eq!(d2.as_millis(), 200);
        assert_eq!(d3.as_millis(), 400);
    }

    #[test]
    fn circuit_opens_after_threshold() {
        let agent = ControlAgent::new("http://localhost:1").with_resilience(ResilienceConfig {
            max_retries: 0,
            base_backoff_ms: 10,
            failure_threshold: 2,
            circuit_cooldown_secs: 10,
        });

        agent.mark_transient_failure();
        assert!(agent.ensure_circuit_closed().is_ok());
        agent.mark_transient_failure();
        assert!(agent.ensure_circuit_closed().is_err());
    }
}
