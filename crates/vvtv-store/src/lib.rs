use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use vvtv_types::{AssetItem, AuditEvent, PipelineMetrics, PlanItem, QueueEntry};

#[derive(Debug, Clone)]
pub struct RecoveredState {
    pub plans: Vec<PlanItem>,
    pub assets: Vec<AssetItem>,
    pub queue: Vec<QueueEntry>,
    pub audits: Vec<AuditEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchedulerCursors {
    pub last_discovery_hour: Option<String>,
    pub last_commit_slot: Option<String>,
    pub last_nightly_date: Option<String>,
}

pub struct StateStore {
    conn: Connection,
}

#[derive(Debug, Clone, Default)]
pub struct ReportData {
    pub plans: Vec<PlanItem>,
    pub assets: Vec<AssetItem>,
    pub audits: Vec<AuditEvent>,
    pub metrics: Vec<PipelineMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertStateRecord {
    pub code: String,
    pub active: bool,
    pub last_notified_at: Option<String>,
    pub updated_at: String,
}

impl StateStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed creating state dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed opening sqlite at {}", path.display()))?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn save_plans(&mut self, plans: &[PlanItem]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for plan in plans {
            tx.execute(
                "INSERT OR REPLACE INTO plans(plan_id, payload_json, updated_at)
                 VALUES(?1, ?2, ?3)",
                params![
                    plan.plan_id,
                    serde_json::to_string(plan)?,
                    plan.discovered_at.to_rfc3339()
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn save_assets(&mut self, assets: &[AssetItem]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for asset in assets {
            tx.execute(
                "INSERT OR REPLACE INTO assets(asset_id, payload_json, updated_at)
                 VALUES(?1, ?2, datetime('now'))",
                params![asset.asset_id, serde_json::to_string(asset)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn replace_queue(&mut self, queue: &[QueueEntry]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM queue_entries", [])?;
        for entry in queue {
            tx.execute(
                "INSERT INTO queue_entries(entry_id, start_at, payload_json)
                 VALUES(?1, ?2, ?3)",
                params![
                    entry.entry_id,
                    entry.start_at.to_rfc3339(),
                    serde_json::to_string(entry)?
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn append_audit(&mut self, event: &AuditEvent) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO audit_events(event_id, ts, payload_json)
             VALUES(?1, ?2, ?3)",
            params![
                event.event_id,
                event.ts.to_rfc3339(),
                serde_json::to_string(event)?
            ],
        )?;
        Ok(())
    }

    pub fn save_metrics(&mut self, metrics: &PipelineMetrics) -> Result<()> {
        self.conn.execute(
            "INSERT INTO metric_samples(ts, payload_json)
             VALUES(?1, ?2)",
            params![Utc::now().to_rfc3339(), serde_json::to_string(metrics)?],
        )?;
        Ok(())
    }

    pub fn load_scheduler_cursors(&self) -> Result<SchedulerCursors> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM scheduler_cursors WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let payload: String = row.get(0)?;
            return Ok(serde_json::from_str(&payload)?);
        }
        Ok(SchedulerCursors::default())
    }

    pub fn save_scheduler_cursors(&mut self, cursors: &SchedulerCursors) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO scheduler_cursors(id, payload_json, updated_at)
             VALUES(1, ?1, ?2)",
            params![serde_json::to_string(cursors)?, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn acquire_scheduler_lock(
        &mut self,
        lock_name: &str,
        owner_id: &str,
        ttl_seconds: i64,
    ) -> Result<bool> {
        let now = Utc::now();
        let expires_at = (now + Duration::seconds(ttl_seconds)).to_rfc3339();

        let tx = self.conn.transaction()?;
        let row = tx.query_row(
            "SELECT owner_id, expires_at FROM scheduler_locks WHERE lock_name = ?1",
            [lock_name],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );

        match row {
            Ok((owner, existing_expires)) => {
                let expired = DateTime::parse_from_rfc3339(&existing_expires)
                    .map(|dt| dt.with_timezone(&Utc) <= now)
                    .unwrap_or(true);
                let is_owner = owner == owner_id;
                if expired || is_owner {
                    tx.execute(
                        "UPDATE scheduler_locks
                         SET owner_id = ?2, acquired_at = ?3, expires_at = ?4
                         WHERE lock_name = ?1",
                        params![lock_name, owner_id, now.to_rfc3339(), expires_at],
                    )?;
                    tx.commit()?;
                    Ok(true)
                } else {
                    tx.commit()?;
                    Ok(false)
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                tx.execute(
                    "INSERT INTO scheduler_locks(lock_name, owner_id, acquired_at, expires_at)
                     VALUES(?1, ?2, ?3, ?4)",
                    params![lock_name, owner_id, now.to_rfc3339(), expires_at],
                )?;
                tx.commit()?;
                Ok(true)
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn release_scheduler_lock(&mut self, lock_name: &str, owner_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM scheduler_locks WHERE lock_name = ?1 AND owner_id = ?2",
            params![lock_name, owner_id],
        )?;
        Ok(())
    }

    pub fn load_latest_metrics(&self) -> Result<Option<PipelineMetrics>> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM metric_samples ORDER BY ts DESC LIMIT 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let payload: String = row.get(0)?;
            return Ok(Some(serde_json::from_str(&payload)?));
        }
        Ok(None)
    }

    pub fn load_recent_metrics(&self, limit: usize) -> Result<Vec<PipelineMetrics>> {
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM metric_samples ORDER BY ts DESC LIMIT ?1")?;
        let rows = stmt.query_map([i64::try_from(limit).unwrap_or(100)], |row| {
            row.get::<_, String>(0)
        })?;
        let mut out = Vec::new();
        for payload in rows {
            out.push(serde_json::from_str::<PipelineMetrics>(&payload?)?);
        }
        Ok(out)
    }

    pub fn load_report_data_between(
        &self,
        start_inclusive: DateTime<Utc>,
        end_exclusive: DateTime<Utc>,
    ) -> Result<ReportData> {
        let start = start_inclusive.to_rfc3339();
        let end = end_exclusive.to_rfc3339();
        Ok(ReportData {
            plans: load_json_table_between(
                &self.conn,
                "SELECT payload_json FROM plans WHERE updated_at >= ?1 AND updated_at < ?2 ORDER BY updated_at ASC",
                &start,
                &end,
            )?,
            assets: load_json_table_between(
                &self.conn,
                "SELECT payload_json FROM assets WHERE updated_at >= ?1 AND updated_at < ?2 ORDER BY updated_at ASC",
                &start,
                &end,
            )?,
            audits: load_json_table_between(
                &self.conn,
                "SELECT payload_json FROM audit_events WHERE ts >= ?1 AND ts < ?2 ORDER BY ts ASC",
                &start,
                &end,
            )?,
            metrics: load_json_table_between(
                &self.conn,
                "SELECT payload_json FROM metric_samples WHERE ts >= ?1 AND ts < ?2 ORDER BY ts ASC",
                &start,
                &end,
            )?,
        })
    }

    pub fn load_alert_states(&self) -> Result<Vec<AlertStateRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT code, active, last_notified_at, updated_at
             FROM alert_states",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(AlertStateRecord {
                code: row.get::<_, String>(0)?,
                active: row.get::<_, i64>(1)? != 0,
                last_notified_at: row.get::<_, Option<String>>(2)?,
                updated_at: row.get::<_, String>(3)?,
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn upsert_alert_state(
        &mut self,
        code: &str,
        active: bool,
        last_notified_at: Option<DateTime<Utc>>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO alert_states(code, active, last_notified_at, updated_at)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(code) DO UPDATE SET
                active = excluded.active,
                last_notified_at = excluded.last_notified_at,
                updated_at = excluded.updated_at",
            params![
                code,
                if active { 1_i64 } else { 0_i64 },
                last_notified_at.map(|dt| dt.to_rfc3339()),
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn load_recent_audits(&self, hours: i64) -> Result<Vec<AuditEvent>> {
        let since = (Utc::now() - Duration::hours(hours)).to_rfc3339();
        let mut stmt = self
            .conn
            .prepare("SELECT payload_json FROM audit_events WHERE ts >= ?1 ORDER BY ts ASC")?;
        let rows = stmt.query_map([since], |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for payload in rows {
            out.push(serde_json::from_str::<AuditEvent>(&payload?)?);
        }
        Ok(out)
    }

    pub fn export_audits_json(&self, path: impl AsRef<Path>) -> Result<usize> {
        let audits: Vec<AuditEvent> = load_json_table(
            &self.conn,
            "SELECT payload_json FROM audit_events ORDER BY ts ASC",
        )?;
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed creating export dir {}", parent.display()))?;
        }
        fs::write(path, serde_json::to_string_pretty(&audits)?)
            .with_context(|| format!("failed writing audit export to {}", path.display()))?;
        Ok(audits.len())
    }

    pub fn enforce_retention_days(&mut self, days: i64) -> Result<usize> {
        let cutoff = (Utc::now() - Duration::days(days)).to_rfc3339();
        let tx = self.conn.transaction()?;
        let deleted_audits =
            tx.execute("DELETE FROM audit_events WHERE ts < ?1", [cutoff.clone()])?;
        let deleted_metrics = tx.execute("DELETE FROM metric_samples WHERE ts < ?1", [cutoff])?;
        tx.commit()?;
        Ok(deleted_audits + deleted_metrics)
    }

    pub fn load_recovery(&self) -> Result<RecoveredState> {
        Ok(RecoveredState {
            plans: load_json_table(&self.conn, "SELECT payload_json FROM plans")?,
            assets: load_json_table(&self.conn, "SELECT payload_json FROM assets")?,
            queue: load_json_table(
                &self.conn,
                "SELECT payload_json FROM queue_entries ORDER BY start_at ASC",
            )?,
            audits: load_json_table(
                &self.conn,
                "SELECT payload_json FROM audit_events ORDER BY ts ASC",
            )?,
        })
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS plans (
                plan_id TEXT PRIMARY KEY,
                payload_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS assets (
                asset_id TEXT PRIMARY KEY,
                payload_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS queue_entries (
                entry_id TEXT PRIMARY KEY,
                start_at TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit_events (
                event_id TEXT PRIMARY KEY,
                ts TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS metric_samples (
                ts TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS scheduler_cursors (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                payload_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS scheduler_locks (
                lock_name TEXT PRIMARY KEY,
                owner_id TEXT NOT NULL,
                acquired_at TEXT NOT NULL,
                expires_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS alert_states (
                code TEXT PRIMARY KEY,
                active INTEGER NOT NULL,
                last_notified_at TEXT,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }
}

fn load_json_table<T: for<'de> serde::Deserialize<'de>>(
    conn: &Connection,
    sql: &str,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut out = Vec::new();
    for payload in rows {
        out.push(serde_json::from_str::<T>(&payload?)?);
    }
    Ok(out)
}

fn load_json_table_between<T: for<'de> serde::Deserialize<'de>>(
    conn: &Connection,
    sql: &str,
    start: &str,
    end: &str,
) -> Result<Vec<T>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![start, end], |row| row.get::<_, String>(0))?;

    let mut out = Vec::new();
    for payload in rows {
        out.push(serde_json::from_str::<T>(&payload?)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use vvtv_types::{
        AssetItem, AuditEvent, PipelineMetrics, PlanItem, PlanState, QaStatus, QueueEntry,
        Resolution, SlotType,
    };

    use super::{SchedulerCursors, StateStore};

    fn open_test_store(path: &str) -> StateStore {
        let _ = std::fs::remove_file(path);
        StateStore::open(path).expect("open store")
    }

    #[test]
    fn persists_and_recovers_state() {
        let mut store = open_test_store("runtime/state/test-vvtv.db");

        let plan = PlanItem {
            plan_id: "plan-1".to_string(),
            source_url: "https://example.com/v/1".to_string(),
            source_domain: "example.com".to_string(),
            discovered_at: Utc::now(),
            title: "t".to_string(),
            duration_sec: 10,
            theme_tags: vec!["x".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            selection_reason: "ok".to_string(),
            policy_match_score: 1.0,
            state: PlanState::Scheduled,
        };

        let asset = AssetItem {
            asset_id: "asset-1".to_string(),
            plan_id: "plan-1".to_string(),
            local_path: "runtime/prepared/a.mp4".to_string(),
            checksum: "chk".to_string(),
            resolution: Resolution {
                width: 1280,
                height: 720,
            },
            audio_lufs: -16.0,
            qa_status: QaStatus::Passed,
        };

        let entry = QueueEntry {
            entry_id: "q-1".to_string(),
            asset_id: "asset-1".to_string(),
            start_at: Utc::now(),
            slot_type: SlotType::Main,
            fallback_level: 0,
            curation_trace_id: None,
        };

        let audit = AuditEvent {
            event_id: "e-1".to_string(),
            ts: Utc::now(),
            actor: "system".to_string(),
            module: "test".to_string(),
            action: "save".to_string(),
            before: None,
            after: None,
            decision_score: None,
            reason_code: "OK".to_string(),
        };

        store.save_plans(&[plan]).expect("save plans");
        store.save_assets(&[asset]).expect("save assets");
        store.replace_queue(&[entry]).expect("save queue");
        store.append_audit(&audit).expect("save audit");
        store
            .save_metrics(&PipelineMetrics {
                buffer_minutes: 60,
                plans_created: 1,
                plans_committed: 1,
                qa_pass_rate: 1.0,
                fallback_rate: 0.0,
                curator_actions: 0,
                stream_disruptions: 0,
            })
            .expect("save metrics");

        let recovered = store.load_recovery().expect("load recovery");
        assert!(!recovered.plans.is_empty());
        assert!(!recovered.assets.is_empty());
        assert!(!recovered.queue.is_empty());
        assert!(!recovered.audits.is_empty());
        assert!(
            store
                .load_latest_metrics()
                .expect("latest metrics")
                .is_some()
        );
    }

    #[test]
    fn export_and_retention_work() {
        let mut store = open_test_store("runtime/state/test-vvtv-retention.db");
        let old_event = AuditEvent {
            event_id: "old-e".to_string(),
            ts: Utc::now() - Duration::days(120),
            actor: "system".to_string(),
            module: "old".to_string(),
            action: "old".to_string(),
            before: None,
            after: None,
            decision_score: None,
            reason_code: "OLD".to_string(),
        };
        let new_event = AuditEvent {
            event_id: "new-e".to_string(),
            ts: Utc::now(),
            actor: "system".to_string(),
            module: "new".to_string(),
            action: "new".to_string(),
            before: None,
            after: None,
            decision_score: None,
            reason_code: "NEW".to_string(),
        };
        store.append_audit(&old_event).expect("old append");
        store.append_audit(&new_event).expect("new append");

        let exported = store
            .export_audits_json("runtime/exports/test-audit-export.json")
            .expect("export");
        assert!(exported >= 2);

        let deleted = store.enforce_retention_days(90).expect("retention");
        assert!(deleted >= 1);
        let recent = store.load_recent_audits(24 * 365).expect("recent");
        assert!(recent.iter().any(|a| a.event_id == "new-e"));
    }

    #[test]
    fn scheduler_cursors_roundtrip() {
        let mut store = open_test_store("runtime/state/test-vvtv-cursors.db");
        let cursors = SchedulerCursors {
            last_discovery_hour: Some("2026-02-27-20".to_string()),
            last_commit_slot: Some("2026-02-27-20-01".to_string()),
            last_nightly_date: Some("2026-02-27".to_string()),
        };
        store
            .save_scheduler_cursors(&cursors)
            .expect("save cursors");
        let loaded = store.load_scheduler_cursors().expect("load cursors");
        assert_eq!(loaded.last_discovery_hour, cursors.last_discovery_hour);
        assert_eq!(loaded.last_commit_slot, cursors.last_commit_slot);
        assert_eq!(loaded.last_nightly_date, cursors.last_nightly_date);
    }

    #[test]
    fn scheduler_lock_enforces_single_owner() {
        let mut store = open_test_store("runtime/state/test-vvtv-lock.db");
        assert!(
            store
                .acquire_scheduler_lock("scheduler-main", "owner-a", 60)
                .expect("owner-a acquires")
        );
        assert!(
            !store
                .acquire_scheduler_lock("scheduler-main", "owner-b", 60)
                .expect("owner-b blocked")
        );
        store
            .release_scheduler_lock("scheduler-main", "owner-a")
            .expect("release");
        assert!(
            store
                .acquire_scheduler_lock("scheduler-main", "owner-b", 60)
                .expect("owner-b acquires after release")
        );
    }

    #[test]
    fn alert_state_roundtrip() {
        let mut store = open_test_store("runtime/state/test-vvtv-alert-state.db");
        let now = Utc::now();
        store
            .upsert_alert_state("BUFFER_CRITICAL", true, Some(now))
            .expect("upsert alert");
        let loaded = store.load_alert_states().expect("load alert states");
        assert!(
            loaded
                .iter()
                .any(|a| a.code == "BUFFER_CRITICAL" && a.active)
        );
    }
}
