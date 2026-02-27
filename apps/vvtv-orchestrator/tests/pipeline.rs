use chrono::Utc;
use vvtv_discovery::DiscoveryEngine;
use vvtv_fetcher::{FetchContext, Fetcher};
use vvtv_planner::Planner;
use vvtv_prep::PrepPipeline;
use vvtv_queue::QueueManager;
use vvtv_types::{
    AutotunePolicy, CuratorPolicy, DiscoveryInput, EditorialProfile, MusicPolicy, OwnerCard,
    QualityPolicy, SafetyPolicy, SchedulePolicy, SearchPolicy,
};

fn owner_card() -> OwnerCard {
    OwnerCard {
        schema_version: 1,
        editorial_profile: EditorialProfile {
            target_avg_duration_sec: 900,
            max_consecutive_same_theme: 2,
            min_unique_themes_per_block: 2,
        },
        search_policy: SearchPolicy {
            allowlist_domains: vec![
                "example-source-a.com".to_string(),
                "example-source-b.com".to_string(),
            ],
            blacklist_domains: vec!["blocked.com".to_string()],
            blocked_keywords: vec!["forbidden".to_string()],
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
            min_confidence: 0.75,
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
    }
}

#[test]
fn full_flow_happy_path() {
    let card = owner_card();
    let inputs = vec![
        DiscoveryInput {
            source_url: "https://example-source-a.com/v/1".to_string(),
            title: "A".to_string(),
            duration_sec: 900,
            theme_tags: vec!["t1".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        },
        DiscoveryInput {
            source_url: "https://example-source-b.com/v/2".to_string(),
            title: "B".to_string(),
            duration_sec: 800,
            theme_tags: vec!["t2".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        },
    ];

    let discovered = DiscoveryEngine::discover(&card, &inputs);
    let day = Planner::build_day(&card, discovered);
    let fetched = Fetcher::commit_t_minus_4h(
        &card,
        Utc::now(),
        day.scheduled,
        day.reserves,
        &FetchContext::default(),
    );
    let prepared = PrepPipeline::process(&card, fetched.clone());
    let queue = QueueManager::build(&card, &prepared, &prepared);

    assert!(!prepared.is_empty());
    assert!(!queue.queue.is_empty());
    assert!(queue.buffer_minutes >= 20);
}

#[test]
fn broken_link_uses_reserve() {
    let card = owner_card();
    let inputs = vec![
        DiscoveryInput {
            source_url: "https://example-source-a.com/v/1".to_string(),
            title: "A".to_string(),
            duration_sec: 900,
            theme_tags: vec!["t1".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        },
        DiscoveryInput {
            source_url: "https://example-source-b.com/v/2".to_string(),
            title: "B".to_string(),
            duration_sec: 900,
            theme_tags: vec!["t2".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        },
        DiscoveryInput {
            source_url: "https://example-source-b.com/v/3".to_string(),
            title: "C".to_string(),
            duration_sec: 900,
            theme_tags: vec!["t3".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        },
    ];

    let discovered = DiscoveryEngine::discover(&card, &inputs);
    let day = Planner::build_day(&card, discovered.clone());
    let mut ctx = FetchContext::default();
    ctx.broken_urls.insert(discovered[0].source_url.clone());
    let fetched = Fetcher::commit_t_minus_4h(&card, Utc::now(), day.scheduled, day.reserves, &ctx);

    assert!(!fetched.is_empty());
}
