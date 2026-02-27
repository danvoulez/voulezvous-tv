use chrono::Utc;
use uuid::Uuid;
use vvtv_types::{DiscoveryInput, OwnerCard, PlanItem, PlanState};

pub struct DiscoveryEngine;

impl DiscoveryEngine {
    #[must_use]
    pub fn discover(owner_card: &OwnerCard, candidates: &[DiscoveryInput]) -> Vec<PlanItem> {
        candidates
            .iter()
            .filter(|candidate| {
                let allowed = owner_card
                    .search_policy
                    .allowlist_domains
                    .iter()
                    .any(|domain| candidate.source_url.contains(domain));
                let blocked_domain = owner_card
                    .search_policy
                    .blacklist_domains
                    .iter()
                    .any(|domain| candidate.source_url.contains(domain));
                let blocked_keyword =
                    owner_card
                        .search_policy
                        .blocked_keywords
                        .iter()
                        .any(|keyword| {
                            candidate
                                .title
                                .to_lowercase()
                                .contains(&keyword.to_lowercase())
                        });
                let hd_ok = !owner_card.safety_policy.require_hd_playback_confirmation
                    || candidate.hd_confirmed;
                allowed && !blocked_domain && !blocked_keyword && hd_ok
            })
            .map(|candidate| PlanItem {
                plan_id: Uuid::new_v4().to_string(),
                source_url: candidate.source_url.clone(),
                source_domain: extract_domain(&candidate.source_url),
                discovered_at: Utc::now(),
                title: candidate.title.clone(),
                duration_sec: candidate.duration_sec,
                theme_tags: candidate.theme_tags.clone(),
                visual_features: candidate.visual_features.clone(),
                quality_signals: candidate.quality_signals.clone(),
                selection_reason: "allowlist+hd-validated".to_string(),
                policy_match_score: 0.85,
                state: PlanState::Candidate,
            })
            .collect()
    }
}

fn extract_domain(url: &str) -> String {
    url.split('/')
        .nth(2)
        .unwrap_or("unknown-domain")
        .to_string()
}
