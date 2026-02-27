use std::path::Path;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Local, Timelike, Utc};
use tracing::info;
use vvtv_audit::{AuditSink, InMemoryAuditSink};
use vvtv_config::OwnerCardStore;
use vvtv_control_agent::{ControlAgent, ResilienceConfig};
use vvtv_curator::Curator;
use vvtv_discovery::DiscoveryEngine;
use vvtv_fetcher::{FetchContext, Fetcher};
use vvtv_nightly::Nightly;
use vvtv_planner::Planner;
use vvtv_prep::PrepPipeline;
use vvtv_queue::QueueManager;
use vvtv_store::{SchedulerCursors, StateStore};
use vvtv_stream::HlsStreamer;
use vvtv_types::{
    AuditEvent, DailyReport, DiscoveryInput, PipelineMetrics, PlanState, WeeklyReport,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter("info")
        .init();
    validate_startup_config()?;

    let owner_card_store =
        OwnerCardStore::load_from_path(Path::new("config/owner_card.sample.yaml"))?;
    let owner_card = owner_card_store.current();
    let mut store = StateStore::open("runtime/state/vvtv.db")?;
    let audit = InMemoryAuditSink::new();
    let cloud_agent = build_cloud_agent()?;
    let instance_id = uuid::Uuid::new_v4().to_string();
    let lock_name = "scheduler-main";
    let lock_ttl_seconds = 30;
    let mut did_boot_recovery = false;

    let run_once = std::env::var("VVTV_RUN_ONCE").ok().as_deref() == Some("1");

    loop {
        let has_lock = store.acquire_scheduler_lock(lock_name, &instance_id, lock_ttl_seconds)?;
        if !has_lock {
            info!(instance_id, "scheduler-standby-lock-not-acquired");
            if run_once {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            continue;
        }

        if !did_boot_recovery {
            try_recover_on_boot(&mut store, &audit)?;
            did_boot_recovery = true;
        }

        let mut cursors = store.load_scheduler_cursors()?;
        let now = Utc::now();
        let local_now = Local::now();

        if due_discovery(now, &cursors) {
            run_discovery_window(&owner_card, &mut store, &audit)?;
            cursors.last_discovery_hour = Some(hour_key(now));
            store.save_scheduler_cursors(&cursors)?;
        }

        if due_commit(
            now,
            owner_card.schedule_policy.commit_interval_minutes,
            &cursors,
        ) {
            run_commit_window(&owner_card, &mut store, &audit, cloud_agent.as_ref()).await?;
            cursors.last_commit_slot = Some(commit_slot_key(
                now,
                owner_card.schedule_policy.commit_interval_minutes,
            ));
            store.save_scheduler_cursors(&cursors)?;
        }

        if due_nightly(local_now, &cursors) {
            run_nightly(&owner_card, &mut store, &audit, cloud_agent.as_ref()).await?;
            cursors.last_nightly_date = Some(date_key(local_now));
            store.save_scheduler_cursors(&cursors)?;
        }

        if run_once {
            store.release_scheduler_lock(lock_name, &instance_id)?;
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
    }

    Ok(())
}

fn build_cloud_agent() -> Result<Option<ControlAgent>> {
    let base_url = match std::env::var("VVTV_CLOUDFLARE_BASE_URL") {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let token = std::env::var("VVTV_CLOUDFLARE_TOKEN").ok();
    let secret = std::env::var("VVTV_CLOUDFLARE_SECRET").ok();

    let mut agent = ControlAgent::new(base_url).with_resilience(ResilienceConfig {
        max_retries: 3,
        base_backoff_ms: 300,
        failure_threshold: 3,
        circuit_cooldown_secs: 30,
    });
    if let (Some(token), Some(secret)) = (token, secret) {
        agent = agent.with_auth(token, secret);
    }
    Ok(Some(agent))
}

fn validate_startup_config() -> Result<()> {
    let env = std::env::var("VVTV_ENV").unwrap_or_else(|_| "dev".to_string());
    if env == "dev" {
        return Ok(());
    }

    let has_base = std::env::var("VVTV_CLOUDFLARE_BASE_URL").is_ok();
    let has_token = std::env::var("VVTV_CLOUDFLARE_TOKEN").is_ok();
    let has_secret = std::env::var("VVTV_CLOUDFLARE_SECRET").is_ok();
    if has_base && (!has_token || !has_secret) {
        return Err(anyhow!(
            "VVTV_CLOUDFLARE_TOKEN and VVTV_CLOUDFLARE_SECRET are required when VVTV_CLOUDFLARE_BASE_URL is set in non-dev"
        ));
    }
    if !has_base {
        return Err(anyhow!(
            "VVTV_CLOUDFLARE_BASE_URL is required when VVTV_ENV != dev"
        ));
    }
    Ok(())
}

fn try_recover_on_boot(store: &mut StateStore, audit: &InMemoryAuditSink) -> Result<()> {
    let recovered = store.load_recovery()?;
    if recovered.queue.is_empty() || recovered.assets.is_empty() {
        return Ok(());
    }

    let hls_output = HlsStreamer::build_hls(&recovered.queue, &recovered.assets, "runtime/hls")?;
    let playlist = std::fs::read_to_string(&hls_output.playlist_path)
        .unwrap_or_else(|_| HlsStreamer::render_playlist(&recovered.queue));

    let qa_passed = recovered
        .assets
        .iter()
        .filter(|a| a.qa_status == vvtv_types::QaStatus::Passed)
        .count();
    let metrics = PipelineMetrics {
        buffer_minutes: (recovered.queue.len() as i64) * 10,
        plans_created: recovered.plans.len(),
        plans_committed: recovered.assets.len(),
        qa_pass_rate: if recovered.assets.is_empty() {
            0.0
        } else {
            qa_passed as f32 / recovered.assets.len() as f32
        },
        fallback_rate: 0.0,
        curator_actions: recovered
            .queue
            .iter()
            .filter(|q| q.curation_trace_id.is_some())
            .count(),
        stream_disruptions: 0,
    };
    store.save_metrics(&metrics)?;

    record_audit(
        audit,
        store,
        audit_event(
            "vvtv-orchestrator",
            "recover",
            "RECOVERY_APPLIED",
            Some(metrics.buffer_minutes as f32),
        ),
    )?;

    info!(
        recovery = true,
        plans_created = metrics.plans_created,
        plans_committed = metrics.plans_committed,
        buffer_minutes = metrics.buffer_minutes,
        qa_pass_rate = metrics.qa_pass_rate,
        playlist_bytes = playlist.len(),
        hls_playlist_path = %hls_output.playlist_path.display(),
        audit_events = audit.list().len(),
        "pipeline-recovered-from-sqlite"
    );

    Ok(())
}

fn run_discovery_window(
    owner_card: &vvtv_types::OwnerCard,
    store: &mut StateStore,
    audit: &InMemoryAuditSink,
) -> Result<()> {
    let discovered = DiscoveryEngine::discover(owner_card, &seed_discovery_inputs());
    let day = Planner::build_day(owner_card, discovered);
    let mut all_plans = day.scheduled;
    all_plans.extend(day.reserves);
    store.save_plans(&all_plans)?;

    record_audit(
        audit,
        store,
        audit_event(
            "vvtv-discovery",
            "discover-window",
            "DISCOVERY_WINDOW_OK",
            Some(all_plans.len() as f32),
        ),
    )?;

    info!(plans_created = all_plans.len(), "discovery-window-complete");
    Ok(())
}

async fn run_commit_window(
    owner_card: &vvtv_types::OwnerCard,
    store: &mut StateStore,
    audit: &InMemoryAuditSink,
    cloud_agent: Option<&ControlAgent>,
) -> Result<()> {
    let recovered = store.load_recovery()?;
    if recovered.plans.is_empty() {
        run_discovery_window(owner_card, store, audit)?;
    }

    let refreshed = store.load_recovery()?;
    let scheduled: Vec<_> = refreshed
        .plans
        .iter()
        .filter(|p| p.state == PlanState::Scheduled)
        .cloned()
        .collect();
    let reserves: Vec<_> = refreshed
        .plans
        .iter()
        .filter(|p| p.state == PlanState::Reserved)
        .cloned()
        .collect();

    let fetched = Fetcher::commit_t_minus_4h(
        owner_card,
        Utc::now(),
        scheduled,
        reserves,
        &FetchContext::default(),
    );
    let prepared = PrepPipeline::process(owner_card, fetched.clone());
    store.save_assets(&prepared)?;

    let queue_result = QueueManager::build(owner_card, &prepared, &prepared);
    let curated = Curator::auto_curate(owner_card, queue_result.queue);
    store.replace_queue(&curated.queue)?;

    let hls_output = HlsStreamer::build_hls(&curated.queue, &prepared, "runtime/hls")?;
    let playlist = std::fs::read_to_string(&hls_output.playlist_path)
        .unwrap_or_else(|_| HlsStreamer::render_playlist(&curated.queue));
    let qa_passed = prepared
        .iter()
        .filter(|a| a.qa_status == vvtv_types::QaStatus::Passed)
        .count();

    let metrics = PipelineMetrics {
        buffer_minutes: queue_result.buffer_minutes,
        plans_created: refreshed.plans.len(),
        plans_committed: fetched.len(),
        qa_pass_rate: if prepared.is_empty() {
            0.0
        } else {
            qa_passed as f32 / prepared.len() as f32
        },
        fallback_rate: if refreshed.plans.is_empty() {
            0.0
        } else {
            (refreshed.plans.len().saturating_sub(fetched.len())) as f32
                / refreshed.plans.len() as f32
        },
        curator_actions: curated.actions_applied,
        stream_disruptions: usize::from(queue_result.emergency_triggered),
    };
    store.save_metrics(&metrics)?;

    record_audit(
        audit,
        store,
        audit_event(
            "vvtv-commit",
            "commit-window",
            "COMMIT_WINDOW_OK",
            Some(metrics.plans_committed as f32),
        ),
    )?;

    info!(
        recovery = false,
        plans_created = metrics.plans_created,
        plans_committed = metrics.plans_committed,
        buffer_minutes = metrics.buffer_minutes,
        qa_pass_rate = metrics.qa_pass_rate,
        fallback_rate = metrics.fallback_rate,
        curator_actions = metrics.curator_actions,
        stream_disruptions = metrics.stream_disruptions,
        playlist_segments = curated.queue.len(),
        playlist_bytes = playlist.len(),
        hls_playlist_path = %hls_output.playlist_path.display(),
        hls_segment_count_estimate = hls_output.segment_count_estimate,
        audit_events = audit.list().len(),
        "commit-window-complete"
    );

    if let Some(agent) = cloud_agent {
        match agent.publish_status_snapshot(&metrics).await {
            Ok(_) => info!("cloudflare-status-sync-ok"),
            Err(err) => info!(error = %err, "cloudflare-status-sync-failed"),
        }
    }

    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}

async fn run_nightly(
    owner_card: &vvtv_types::OwnerCard,
    store: &mut StateStore,
    audit: &InMemoryAuditSink,
    cloud_agent: Option<&ControlAgent>,
) -> Result<()> {
    let recovered = store.load_recovery()?;
    let qa_passed = recovered
        .assets
        .iter()
        .filter(|a| a.qa_status == vvtv_types::QaStatus::Passed)
        .count();
    let metrics = PipelineMetrics {
        buffer_minutes: (recovered.queue.len() as i64) * 10,
        plans_created: recovered.plans.len(),
        plans_committed: recovered.assets.len(),
        qa_pass_rate: if recovered.assets.is_empty() {
            0.0
        } else {
            qa_passed as f32 / recovered.assets.len() as f32
        },
        fallback_rate: 0.0,
        curator_actions: recovered
            .queue
            .iter()
            .filter(|q| q.curation_trace_id.is_some())
            .count(),
        stream_disruptions: 0,
    };
    store.save_metrics(&metrics)?;

    let nightly_action = Nightly::tune(owner_card, &metrics);
    record_audit(
        audit,
        store,
        audit_event("vvtv-nightly", "nightly-job", &nightly_action, None),
    )?;

    let export_path = format!(
        "runtime/exports/audit-{}.json",
        Utc::now().format("%Y-%m-%d")
    );
    let exported_count = store.export_audits_json(export_path)?;
    let deleted_count = store.enforce_retention_days(90)?;

    if let Some(agent) = cloud_agent {
        let daily = DailyReport {
            date: Utc::now().format("%Y-%m-%d").to_string(),
            summary: format!("nightly export_count={exported_count} deleted_count={deleted_count}"),
            metrics: metrics.clone(),
        };
        let weekly = WeeklyReport {
            week: Utc::now().format("%G-W%V").to_string(),
            summary: "weekly sync snapshot".to_string(),
            metrics: metrics.clone(),
        };

        match agent.publish_daily_report(&daily).await {
            Ok(_) => info!("cloudflare-daily-sync-ok"),
            Err(err) => info!(error = %err, "cloudflare-daily-sync-failed"),
        }
        match agent.publish_weekly_report(&weekly).await {
            Ok(_) => info!("cloudflare-weekly-sync-ok"),
            Err(err) => info!(error = %err, "cloudflare-weekly-sync-failed"),
        }
    }

    info!(
        nightly_action,
        exported_count, deleted_count, "nightly-job-complete"
    );
    Ok(())
}

fn record_audit(
    audit: &InMemoryAuditSink,
    store: &mut StateStore,
    event: AuditEvent,
) -> Result<()> {
    audit.append(event.clone());
    store.append_audit(&event)?;
    Ok(())
}

fn seed_discovery_inputs() -> Vec<DiscoveryInput> {
    vec![
        DiscoveryInput {
            source_url: "https://example-source-a.com/video/1".to_string(),
            title: "Night Session A".to_string(),
            duration_sec: 900,
            theme_tags: vec!["noir".to_string(), "night".to_string()],
            visual_features: vec!["low-light".to_string()],
            quality_signals: vec!["1080p".to_string(), "clean-audio".to_string()],
            hd_confirmed: true,
        },
        DiscoveryInput {
            source_url: "https://example-source-b.com/video/2".to_string(),
            title: "Studio Flow".to_string(),
            duration_sec: 780,
            theme_tags: vec!["studio".to_string()],
            visual_features: vec!["close-up".to_string()],
            quality_signals: vec!["720p".to_string()],
            hd_confirmed: true,
        },
        DiscoveryInput {
            source_url: "https://bad-domain.com/video/3".to_string(),
            title: "Blocked Source".to_string(),
            duration_sec: 600,
            theme_tags: vec!["rejected".to_string()],
            visual_features: vec!["unknown".to_string()],
            quality_signals: vec!["480p".to_string()],
            hd_confirmed: true,
        },
    ]
}

fn audit_event(module: &str, action: &str, reason_code: &str, score: Option<f32>) -> AuditEvent {
    AuditEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        ts: Utc::now(),
        actor: "system".to_string(),
        module: module.to_string(),
        action: action.to_string(),
        before: None,
        after: None,
        decision_score: score,
        reason_code: reason_code.to_string(),
    }
}

fn hour_key(ts: DateTime<Utc>) -> String {
    format!(
        "{:04}-{:02}-{:02}-{:02}",
        ts.year(),
        ts.month(),
        ts.day(),
        ts.hour()
    )
}

fn commit_slot_key(ts: DateTime<Utc>, interval_minutes: u16) -> String {
    let interval = u32::from(interval_minutes.max(1));
    let slot = ts.minute() / interval;
    format!(
        "{:04}-{:02}-{:02}-{:02}-{:02}",
        ts.year(),
        ts.month(),
        ts.day(),
        ts.hour(),
        slot
    )
}

fn date_key(ts: DateTime<Local>) -> String {
    format!("{:04}-{:02}-{:02}", ts.year(), ts.month(), ts.day())
}

fn due_discovery(now: DateTime<Utc>, cursors: &SchedulerCursors) -> bool {
    let key = hour_key(now);
    cursors.last_discovery_hour.as_deref() != Some(key.as_str())
}

fn due_commit(now: DateTime<Utc>, interval_minutes: u16, cursors: &SchedulerCursors) -> bool {
    let key = commit_slot_key(now, interval_minutes);
    cursors.last_commit_slot.as_deref() != Some(key.as_str())
}

fn due_nightly(now: DateTime<Local>, cursors: &SchedulerCursors) -> bool {
    if std::env::var("VVTV_FORCE_NIGHTLY").ok().as_deref() == Some("1") {
        return true;
    }
    let date = date_key(now);
    if cursors.last_nightly_date.as_deref() == Some(date.as_str()) {
        return false;
    }
    now.hour() == 3
}

use chrono::Datelike;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    #[test]
    fn commit_slot_changes_by_interval() {
        let t1 = Utc.with_ymd_and_hms(2026, 2, 27, 10, 5, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 2, 27, 10, 29, 0).unwrap();
        let t3 = Utc.with_ymd_and_hms(2026, 2, 27, 10, 30, 0).unwrap();

        assert_eq!(commit_slot_key(t1, 30), commit_slot_key(t2, 30));
        assert_ne!(commit_slot_key(t2, 30), commit_slot_key(t3, 30));
    }

    #[test]
    fn nightly_runs_once_per_day() {
        let naive = NaiveDate::from_ymd_opt(2026, 2, 27)
            .unwrap()
            .and_hms_opt(3, 10, 0)
            .unwrap();
        let local_time = Local.from_local_datetime(&naive).single().unwrap();
        let mut cursors = SchedulerCursors::default();

        assert!(due_nightly(local_time, &cursors));
        cursors.last_nightly_date = Some(date_key(local_time));
        assert!(!due_nightly(local_time, &cursors));
    }
}
