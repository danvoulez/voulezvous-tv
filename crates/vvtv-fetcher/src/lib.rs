use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;
use vvtv_types::{AssetItem, OwnerCard, PlanItem, QaStatus, Resolution};

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
        let target_items =
            usize::from((owner_card.schedule_policy.buffer_target_minutes / 10).max(1));
        let mut assets = Vec::new();
        let mut used_plan_ids = HashSet::new();

        for item in scheduled {
            if !eligible_for_commit(&item, cutoff, ctx) {
                continue;
            }
            used_plan_ids.insert(item.plan_id.clone());
            assets.push(to_asset(&item));
            if assets.len() >= target_items {
                return assets;
            }
        }

        for reserve in reserves {
            if used_plan_ids.contains(&reserve.plan_id)
                || !eligible_for_commit(&reserve, cutoff, ctx)
            {
                continue;
            }
            used_plan_ids.insert(reserve.plan_id.clone());
            assets.push(to_asset(&reserve));
            if assets.len() >= target_items {
                break;
            }
        }

        assets
    }
}

fn eligible_for_commit(item: &PlanItem, cutoff: DateTime<Utc>, ctx: &FetchContext) -> bool {
    item.discovered_at <= cutoff && !ctx.broken_urls.contains(&item.source_url)
}

fn to_asset(plan: &PlanItem) -> AssetItem {
    let vertical = plan
        .visual_features
        .iter()
        .any(|feature| feature.to_lowercase().contains("vertical"));
    let (width, height) = if vertical { (720, 1280) } else { (1280, 720) };

    AssetItem {
        asset_id: Uuid::new_v4().to_string(),
        plan_id: plan.plan_id.clone(),
        local_path: format!("/var/vvtv/assets/{}.mp4", plan.plan_id),
        checksum: format!("chk-{}", &plan.plan_id[..plan.plan_id.len().min(8)]),
        resolution: Resolution { width, height },
        audio_lufs: -19.0,
        qa_status: QaStatus::Pending,
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use vvtv_types::{
        AutotunePolicy, CuratorPolicy, EditorialProfile, MusicPolicy, OwnerCard, PlanItem,
        PlanState, QualityPolicy, SafetyPolicy, SchedulePolicy, SearchPolicy,
    };

    use super::*;

    #[test]
    fn commit_backfills_with_reserves_to_target_buffer() {
        let card = sample_card(40);
        let now = Utc::now();
        let scheduled = vec![sample_plan("a", now), sample_plan("b", now)];
        let reserves = vec![
            sample_plan("r1", now),
            sample_plan("r2", now),
            sample_plan("r3", now),
        ];

        let assets =
            Fetcher::commit_t_minus_4h(&card, now, scheduled, reserves, &FetchContext::default());

        assert_eq!(assets.len(), 4);
    }

    #[test]
    fn commit_skips_broken_urls() {
        let card = sample_card(20);
        let now = Utc::now();
        let bad = sample_plan("bad", now);
        let good = sample_plan("good", now);
        let mut ctx = FetchContext::default();
        ctx.broken_urls.insert(bad.source_url.clone());

        let assets = Fetcher::commit_t_minus_4h(&card, now, vec![bad, good.clone()], vec![], &ctx);

        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].plan_id, good.plan_id);
    }

    #[test]
    fn commit_rejects_future_discoveries() {
        let card = sample_card(20);
        let now = Utc::now();
        let future = sample_plan("future", now + Duration::hours(8));

        let assets =
            Fetcher::commit_t_minus_4h(&card, now, vec![future], vec![], &FetchContext::default());
        assert!(assets.is_empty());
    }

    fn sample_card(buffer_target_minutes: u16) -> OwnerCard {
        OwnerCard {
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
                buffer_target_minutes,
                buffer_critical_minutes: 20,
            },
            quality_policy: QualityPolicy {
                min_resolution_height: 720,
                target_audio_lufs: -16.0,
                max_audio_deviation_lufs: 2.5,
            },
            music_policy: MusicPolicy {
                preferred_moods: vec![],
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
                max_daily_adjustment_pct: 10.0,
                enabled: true,
            },
        }
    }

    fn sample_plan(id: &str, discovered_at: chrono::DateTime<Utc>) -> PlanItem {
        PlanItem {
            plan_id: id.to_string(),
            source_url: format!("https://example.com/{id}"),
            source_domain: "example.com".to_string(),
            discovered_at,
            title: format!("title-{id}"),
            duration_sec: 900,
            theme_tags: vec!["theme-a".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            selection_reason: "test".to_string(),
            policy_match_score: 0.95,
            state: PlanState::Scheduled,
        }
    }
}
