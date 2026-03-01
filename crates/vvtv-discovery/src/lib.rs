use chrono::Utc;
use uuid::Uuid;
use vvtv_types::{DiscoveryInput, OwnerCard, PlanItem, PlanState};

pub struct DiscoveryEngine;

impl DiscoveryEngine {
    #[must_use]
    pub fn discover(owner_card: &OwnerCard, candidates: &[DiscoveryInput]) -> Vec<PlanItem> {
        let mut accepted: Vec<PlanItem> = candidates
            .iter()
            .filter_map(|candidate| map_candidate(owner_card, candidate))
            .collect();

        accepted.sort_by(|a, b| {
            b.policy_match_score
                .total_cmp(&a.policy_match_score)
                .then_with(|| b.discovered_at.cmp(&a.discovered_at))
        });
        accepted
    }
}

fn map_candidate(owner_card: &OwnerCard, candidate: &DiscoveryInput) -> Option<PlanItem> {
    let source_domain = extract_domain(&candidate.source_url);
    if !is_allowlisted(owner_card, &source_domain)
        || is_blocked_domain(owner_card, &source_domain)
        || has_blocked_keyword(owner_card, candidate)
    {
        return None;
    }

    if owner_card.safety_policy.require_hd_playback_confirmation && !candidate.hd_confirmed {
        return None;
    }

    let score = policy_score(owner_card, candidate);
    let selection_reason = format!(
        "allowlisted domain={} hd={} score={:.2}",
        source_domain, candidate.hd_confirmed, score
    );

    Some(PlanItem {
        plan_id: Uuid::new_v4().to_string(),
        source_url: candidate.source_url.clone(),
        source_domain,
        discovered_at: Utc::now(),
        title: candidate.title.clone(),
        duration_sec: candidate.duration_sec,
        theme_tags: candidate.theme_tags.clone(),
        visual_features: candidate.visual_features.clone(),
        quality_signals: candidate.quality_signals.clone(),
        selection_reason,
        policy_match_score: score,
        state: PlanState::Candidate,
    })
}

fn policy_score(owner_card: &OwnerCard, candidate: &DiscoveryInput) -> f32 {
    let mut score = 0.5;

    if candidate.duration_sec > 60 {
        let target = owner_card.editorial_profile.target_avg_duration_sec as i64;
        let diff = (i64::from(candidate.duration_sec) - target).unsigned_abs() as f32;
        let normalized = (1.0 - (diff / target.max(1) as f32)).clamp(0.0, 1.0);
        score += normalized * 0.2;
    }

    let mood_matches = candidate
        .theme_tags
        .iter()
        .filter(|tag| {
            owner_card
                .music_policy
                .preferred_moods
                .iter()
                .any(|mood| mood.eq_ignore_ascii_case(tag))
        })
        .count();
    score += (mood_matches as f32 * 0.08).min(0.16);

    let quality_hits = candidate
        .quality_signals
        .iter()
        .filter(|q| {
            let lower = q.to_lowercase();
            lower.contains("4k")
                || lower.contains("1080")
                || lower.contains("stereo")
                || lower.contains("clean")
        })
        .count();
    score += (quality_hits as f32 * 0.05).min(0.15);

    score.clamp(0.0, 1.0)
}

fn has_blocked_keyword(owner_card: &OwnerCard, candidate: &DiscoveryInput) -> bool {
    let title = candidate.title.to_lowercase();
    let tags = candidate
        .theme_tags
        .iter()
        .map(|t| t.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    owner_card
        .search_policy
        .blocked_keywords
        .iter()
        .map(|k| k.to_lowercase())
        .any(|keyword| title.contains(&keyword) || tags.contains(&keyword))
}

fn is_allowlisted(owner_card: &OwnerCard, source_domain: &str) -> bool {
    owner_card
        .search_policy
        .allowlist_domains
        .iter()
        .any(|domain| source_domain.ends_with(domain))
}

fn is_blocked_domain(owner_card: &OwnerCard, source_domain: &str) -> bool {
    owner_card
        .search_policy
        .blacklist_domains
        .iter()
        .any(|domain| source_domain.ends_with(domain))
}

fn extract_domain(url: &str) -> String {
    url.split("//")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .unwrap_or("unknown-domain")
        .to_string()
}

#[cfg(test)]
mod tests {
    use vvtv_types::{
        AutotunePolicy, CuratorPolicy, EditorialProfile, MusicPolicy, OwnerCard, QualityPolicy,
        SafetyPolicy, SchedulePolicy, SearchPolicy,
    };

    use super::*;

    #[test]
    fn discover_filters_blocked_keywords_on_tags_and_title() {
        let card = sample_owner_card();
        let blocked = DiscoveryInput {
            source_url: "https://media.example.com/x".to_string(),
            title: "cool content".to_string(),
            duration_sec: 900,
            theme_tags: vec!["violence".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        };

        let accepted = DiscoveryEngine::discover(&card, &[blocked]);
        assert!(accepted.is_empty());
    }

    #[test]
    fn discover_scores_preferred_mood_higher() {
        let card = sample_owner_card();
        let plain = DiscoveryInput {
            source_url: "https://media.example.com/a".to_string(),
            title: "plain".to_string(),
            duration_sec: 900,
            theme_tags: vec!["travel".to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            hd_confirmed: true,
        };
        let mood = DiscoveryInput {
            source_url: "https://media.example.com/b".to_string(),
            title: "mood".to_string(),
            duration_sec: 900,
            theme_tags: vec!["night".to_string()],
            visual_features: vec![],
            quality_signals: vec!["1080p".to_string()],
            hd_confirmed: true,
        };

        let accepted = DiscoveryEngine::discover(&card, &[plain, mood]);
        assert_eq!(accepted.len(), 2);
        assert!(accepted[0].policy_match_score >= accepted[1].policy_match_score);
    }

    fn sample_owner_card() -> OwnerCard {
        OwnerCard {
            schema_version: 1,
            editorial_profile: EditorialProfile {
                target_avg_duration_sec: 900,
                max_consecutive_same_theme: 2,
                min_unique_themes_per_block: 3,
            },
            search_policy: SearchPolicy {
                allowlist_domains: vec!["example.com".to_string()],
                blacklist_domains: vec!["evil.example.com".to_string()],
                blocked_keywords: vec!["violence".to_string()],
            },
            schedule_policy: SchedulePolicy {
                planning_horizon_hours: 24,
                commit_lead_hours: 4,
                commit_interval_minutes: 30,
                buffer_target_minutes: 120,
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
                max_daily_adjustment_pct: 10.0,
                enabled: true,
            },
        }
    }
}
