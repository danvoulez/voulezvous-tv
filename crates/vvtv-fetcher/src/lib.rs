use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use vvtv_types::{AssetItem, OwnerCard, PlanItem, PlanState, QaStatus, Resolution};

#[derive(Default)]
pub struct FetchContext {
    pub broken_urls: HashSet<String>,
}

pub struct Fetcher;

impl Fetcher {
    #[must_use]
    pub fn commit_t_minus_4h(
        owner_card: &OwnerCard,
        now: DateTime<Utc>,
        scheduled: Vec<PlanItem>,
        reserves: Vec<PlanItem>,
        ctx: &FetchContext,
    ) -> Vec<AssetItem> {
        let cutoff = now + Duration::hours(i64::from(owner_card.schedule_policy.commit_lead_hours));
        let mut assets = Vec::new();
        let mut reserve_iter = reserves.into_iter();

        for mut item in scheduled {
            if item.discovered_at <= cutoff {
                if ctx.broken_urls.contains(&item.source_url) {
                    if let Some(mut reserve) = reserve_iter.next() {
                        reserve.state = PlanState::Committed;
                        assets.push(to_asset(&reserve));
                    }
                    continue;
                }
                item.state = PlanState::Committed;
                assets.push(to_asset(&item));
            }
        }

        assets
    }
}

fn to_asset(plan: &PlanItem) -> AssetItem {
    AssetItem {
        asset_id: Uuid::new_v4().to_string(),
        plan_id: plan.plan_id.clone(),
        local_path: format!("/var/vvtv/assets/{}.mp4", plan.plan_id),
        checksum: format!("chk-{}", &plan.plan_id[..8]),
        resolution: Resolution {
            width: 1280,
            height: 720,
        },
        audio_lufs: -19.0,
        qa_status: QaStatus::Pending,
    }
}
