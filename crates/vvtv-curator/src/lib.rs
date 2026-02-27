use uuid::Uuid;
use vvtv_types::{OwnerCard, QueueEntry};

pub struct CuratorResult {
    pub queue: Vec<QueueEntry>,
    pub actions_applied: usize,
}

pub struct Curator;

impl Curator {
    #[must_use]
    pub fn auto_curate(owner_card: &OwnerCard, mut queue: Vec<QueueEntry>) -> CuratorResult {
        if !owner_card.curator_policy.auto_apply || queue.len() < 3 {
            return CuratorResult {
                queue,
                actions_applied: 0,
            };
        }

        // Simple anti-repetition move: swap positions 1 and 2 once per cycle.
        queue.swap(1, 2);
        let trace_id = Uuid::new_v4().to_string();
        if let Some(first) = queue.get_mut(1) {
            first.curation_trace_id = Some(trace_id.clone());
        }
        if let Some(second) = queue.get_mut(2) {
            second.curation_trace_id = Some(trace_id);
        }

        CuratorResult {
            queue,
            actions_applied: 1,
        }
    }
}
