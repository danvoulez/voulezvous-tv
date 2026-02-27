use chrono::{Duration, Utc};
use uuid::Uuid;
use vvtv_types::{AssetItem, OwnerCard, QaStatus, QueueEntry, SlotType};

pub struct QueueManager;

pub struct QueueBuildResult {
    pub queue: Vec<QueueEntry>,
    pub emergency_triggered: bool,
    pub buffer_minutes: i64,
}

impl QueueManager {
    #[must_use]
    pub fn build(
        owner_card: &OwnerCard,
        assets: &[AssetItem],
        emergency_pool: &[AssetItem],
    ) -> QueueBuildResult {
        let mut queue = Vec::new();
        let mut cursor = Utc::now();

        for asset in assets.iter().filter(|a| a.qa_status == QaStatus::Passed) {
            queue.push(QueueEntry {
                entry_id: Uuid::new_v4().to_string(),
                asset_id: asset.asset_id.clone(),
                start_at: cursor,
                slot_type: SlotType::Main,
                fallback_level: 0,
                curation_trace_id: None,
            });
            cursor += Duration::minutes(10);
        }

        let mut buffer_minutes = (queue.len() as i64) * 10;
        let mut emergency_triggered = false;

        if buffer_minutes < i64::from(owner_card.schedule_policy.buffer_critical_minutes) {
            emergency_triggered = true;
            for asset in emergency_pool.iter().take(3) {
                queue.push(QueueEntry {
                    entry_id: Uuid::new_v4().to_string(),
                    asset_id: asset.asset_id.clone(),
                    start_at: cursor,
                    slot_type: SlotType::Emergency,
                    fallback_level: 1,
                    curation_trace_id: None,
                });
                cursor += Duration::minutes(10);
            }
            buffer_minutes = (queue.len() as i64) * 10;
        }

        QueueBuildResult {
            queue,
            emergency_triggered,
            buffer_minutes,
        }
    }
}
