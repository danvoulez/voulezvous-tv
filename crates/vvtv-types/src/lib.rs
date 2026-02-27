use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerCard {
    pub schema_version: u16,
    pub editorial_profile: EditorialProfile,
    pub search_policy: SearchPolicy,
    pub schedule_policy: SchedulePolicy,
    pub quality_policy: QualityPolicy,
    pub music_policy: MusicPolicy,
    pub curator_policy: CuratorPolicy,
    pub safety_policy: SafetyPolicy,
    pub autotune_policy: AutotunePolicy,
}

impl OwnerCard {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version == 0 {
            return Err("schema_version must be >= 1".to_string());
        }
        if self.search_policy.allowlist_domains.is_empty() {
            return Err("allowlist_domains cannot be empty".to_string());
        }
        if self.schedule_policy.buffer_critical_minutes
            >= self.schedule_policy.buffer_target_minutes
        {
            return Err(
                "buffer_critical_minutes must be lower than buffer_target_minutes".to_string(),
            );
        }
        if self.autotune_policy.max_daily_adjustment_pct > 20.0 {
            return Err("max_daily_adjustment_pct must be <= 20".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorialProfile {
    pub target_avg_duration_sec: u32,
    pub max_consecutive_same_theme: u8,
    pub min_unique_themes_per_block: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPolicy {
    pub allowlist_domains: Vec<String>,
    pub blacklist_domains: Vec<String>,
    pub blocked_keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulePolicy {
    pub planning_horizon_hours: u16,
    pub commit_lead_hours: u16,
    pub commit_interval_minutes: u16,
    pub buffer_target_minutes: u16,
    pub buffer_critical_minutes: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityPolicy {
    pub min_resolution_height: u16,
    pub target_audio_lufs: f32,
    pub max_audio_deviation_lufs: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicPolicy {
    pub preferred_moods: Vec<String>,
    pub block_music_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratorPolicy {
    pub auto_apply: bool,
    pub min_confidence: f32,
    pub max_reorders_per_hour: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyPolicy {
    pub require_hd_playback_confirmation: bool,
    pub reject_suspicious_watermark: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutotunePolicy {
    pub max_daily_adjustment_pct: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlanState {
    Candidate,
    Reserved,
    Scheduled,
    Committed,
    Dropped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanItem {
    pub plan_id: String,
    pub source_url: String,
    pub source_domain: String,
    pub discovered_at: DateTime<Utc>,
    pub title: String,
    pub duration_sec: u32,
    pub theme_tags: Vec<String>,
    pub visual_features: Vec<String>,
    pub quality_signals: Vec<String>,
    pub selection_reason: String,
    pub policy_match_score: f32,
    pub state: PlanState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QaStatus {
    Pending,
    Passed,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetItem {
    pub asset_id: String,
    pub plan_id: String,
    pub local_path: String,
    pub checksum: String,
    pub resolution: Resolution,
    pub audio_lufs: f32,
    pub qa_status: QaStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotType {
    Main,
    Reserve,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub entry_id: String,
    pub asset_id: String,
    pub start_at: DateTime<Utc>,
    pub slot_type: SlotType,
    pub fallback_level: u8,
    pub curation_trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub ts: DateTime<Utc>,
    pub actor: String,
    pub module: String,
    pub action: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub decision_score: Option<f32>,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryInput {
    pub source_url: String,
    pub title: String,
    pub duration_sec: u32,
    pub theme_tags: Vec<String>,
    pub visual_features: Vec<String>,
    pub quality_signals: Vec<String>,
    pub hd_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedDay {
    pub scheduled: Vec<PlanItem>,
    pub reserves: Vec<PlanItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMetrics {
    pub buffer_minutes: i64,
    pub plans_created: usize,
    pub plans_committed: usize,
    pub qa_pass_rate: f32,
    pub fallback_rate: f32,
    pub curator_actions: usize,
    pub stream_disruptions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReport {
    pub date: String,
    pub summary: String,
    pub metrics: PipelineMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyReport {
    pub week: String,
    pub summary: String,
    pub metrics: PipelineMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn owner_card_validation_works() {
        let card = OwnerCard {
            schema_version: 1,
            editorial_profile: EditorialProfile {
                target_avg_duration_sec: 900,
                max_consecutive_same_theme: 2,
                min_unique_themes_per_block: 3,
            },
            search_policy: SearchPolicy {
                allowlist_domains: vec!["example.com".to_string()],
                blacklist_domains: vec![],
                blocked_keywords: vec![],
            },
            schedule_policy: SchedulePolicy {
                planning_horizon_hours: 24,
                commit_lead_hours: 4,
                commit_interval_minutes: 30,
                buffer_target_minutes: 60,
                buffer_critical_minutes: 20,
            },
            quality_policy: QualityPolicy {
                min_resolution_height: 720,
                target_audio_lufs: -16.0,
                max_audio_deviation_lufs: 2.5,
            },
            music_policy: MusicPolicy {
                preferred_moods: vec!["night".to_string()],
                block_music_ratio: 0.2,
            },
            curator_policy: CuratorPolicy {
                auto_apply: true,
                min_confidence: 0.8,
                max_reorders_per_hour: 4,
            },
            safety_policy: SafetyPolicy {
                require_hd_playback_confirmation: true,
                reject_suspicious_watermark: true,
            },
            autotune_policy: AutotunePolicy {
                max_daily_adjustment_pct: 5.0,
                enabled: true,
            },
        };

        assert!(card.validate().is_ok());
    }

    #[test]
    fn serde_roundtrip_plan_item() {
        let plan = PlanItem {
            plan_id: "p1".to_string(),
            source_url: "https://example.com/video/1".to_string(),
            source_domain: "example.com".to_string(),
            discovered_at: Utc::now(),
            title: "Sample".to_string(),
            duration_sec: 600,
            theme_tags: vec!["night".to_string()],
            visual_features: vec!["contrast".to_string()],
            quality_signals: vec!["hd".to_string()],
            selection_reason: "policy".to_string(),
            policy_match_score: 0.9,
            state: PlanState::Candidate,
        };
        let json = serde_json::to_string(&plan).expect("serialize");
        let back: PlanItem = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.plan_id, "p1");
        assert_eq!(back.state, PlanState::Candidate);
    }
}
