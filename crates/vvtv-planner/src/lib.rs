use std::collections::{HashMap, HashSet, VecDeque};

use vvtv_types::{OwnerCard, PlanItem, PlanState, PlannedDay};

pub struct Planner;

impl Planner {
    #[must_use]
    pub fn build_day(owner_card: &OwnerCard, plans: Vec<PlanItem>) -> PlannedDay {
        let block_unique_target =
            usize::from(owner_card.editorial_profile.min_unique_themes_per_block).max(1);
        let max_consecutive_same_theme =
            usize::from(owner_card.editorial_profile.max_consecutive_same_theme).max(1);
        let target_duration = owner_card.editorial_profile.target_avg_duration_sec;
        let mut scored = plans;

        // Higher score first, then fresher discoveries.
        scored.sort_by(|a, b| {
            b.policy_match_score
                .total_cmp(&a.policy_match_score)
                .then_with(|| b.discovered_at.cmp(&a.discovered_at))
        });

        let mut deduped = Vec::new();
        let mut seen_urls = HashSet::new();
        let mut seen_titles = HashSet::new();
        for plan in scored {
            let title_key = normalize_key(&plan.title);
            if seen_urls.insert(plan.source_url.clone()) && seen_titles.insert(title_key) {
                deduped.push(plan);
            }
        }

        let mut buckets: HashMap<String, VecDeque<PlanItem>> = HashMap::new();
        let mut generic_bucket = VecDeque::new();
        for plan in deduped {
            let theme = primary_theme(&plan);
            if theme == "generic" {
                generic_bucket.push_back(plan);
            } else {
                buckets.entry(theme).or_default().push_back(plan);
            }
        }

        let mut theme_order: Vec<String> = buckets.keys().cloned().collect();
        theme_order.sort();

        let mut scheduled = Vec::new();
        let mut reserves = Vec::new();
        let mut recent_themes: VecDeque<String> = VecDeque::new();
        let mut streak_theme = String::new();
        let mut streak_count = 0usize;
        let mut total_duration = 0u64;

        loop {
            let mut pick_theme = None;
            let mut best_score = f32::MIN;

            for theme in &theme_order {
                let Some(candidate) = buckets.get(theme).and_then(|q| q.front()) else {
                    continue;
                };
                if streak_theme == *theme && streak_count >= max_consecutive_same_theme {
                    continue;
                }

                let candidate_score = fairness_score(
                    candidate,
                    theme,
                    &recent_themes,
                    block_unique_target,
                    target_duration,
                    total_duration,
                    scheduled.len(),
                );
                if candidate_score > best_score {
                    best_score = candidate_score;
                    pick_theme = Some(theme.clone());
                }
            }

            let Some(theme) = pick_theme else {
                break;
            };

            if let Some(plan) = buckets.get_mut(&theme).and_then(VecDeque::pop_front) {
                total_duration += u64::from(plan.duration_sec);
                push_scheduled(
                    plan,
                    &theme,
                    &mut scheduled,
                    &mut recent_themes,
                    block_unique_target,
                    &mut streak_theme,
                    &mut streak_count,
                );
            }
        }

        // Consume generic bucket and leftovers as reserves.
        reserves.extend(generic_bucket.into_iter().map(to_reserved));
        for theme in theme_order {
            if let Some(mut bucket) = buckets.remove(&theme) {
                reserves.extend(bucket.drain(..).map(to_reserved));
            }
        }

        PlannedDay {
            scheduled,
            reserves,
        }
    }
}

fn fairness_score(
    candidate: &PlanItem,
    theme: &str,
    recent_themes: &VecDeque<String>,
    block_unique_target: usize,
    target_duration: u32,
    total_duration: u64,
    scheduled_count: usize,
) -> f32 {
    let mut score = candidate.policy_match_score * 100.0;

    // Encourage theme diversity inside a sliding window.
    let unique_recent = recent_themes.iter().collect::<HashSet<_>>().len();
    let seen_in_recent = recent_themes.iter().any(|t| t == theme);
    if unique_recent < block_unique_target && !seen_in_recent {
        score += 30.0;
    }

    // Keep running duration close to target average.
    let current_avg = if scheduled_count == 0 {
        target_duration as f32
    } else {
        total_duration as f32 / scheduled_count as f32
    };
    let next_avg = (total_duration + u64::from(candidate.duration_sec)) as f32
        / (scheduled_count as f32 + 1.0);
    let current_diff = (current_avg - target_duration as f32).abs();
    let next_diff = (next_avg - target_duration as f32).abs();
    if next_diff < current_diff {
        score += 10.0;
    } else {
        score -= (next_diff - current_diff).min(25.0);
    }

    score
}

fn push_scheduled(
    mut plan: PlanItem,
    theme: &str,
    scheduled: &mut Vec<PlanItem>,
    recent_themes: &mut VecDeque<String>,
    block_unique_target: usize,
    streak_theme: &mut String,
    streak_count: &mut usize,
) {
    plan.state = PlanState::Scheduled;
    scheduled.push(plan);

    if *streak_theme == theme {
        *streak_count += 1;
    } else {
        *streak_theme = theme.to_string();
        *streak_count = 1;
    }

    recent_themes.push_back(theme.to_string());
    if recent_themes.len() > block_unique_target * 2 {
        recent_themes.pop_front();
    }
}

fn to_reserved(mut plan: PlanItem) -> PlanItem {
    plan.state = PlanState::Reserved;
    plan
}

fn primary_theme(plan: &PlanItem) -> String {
    plan.theme_tags
        .iter()
        .find(|theme| !theme.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "generic".to_string())
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use vvtv_types::{
        AutotunePolicy, CuratorPolicy, EditorialProfile, MusicPolicy, OwnerCard, PlanItem,
        QualityPolicy, SafetyPolicy, SchedulePolicy, SearchPolicy,
    };

    use super::*;

    #[test]
    fn planner_enforces_consecutive_theme_limit() {
        let card = sample_card();
        let plans = vec![
            sample_plan("a", "theme-a", 0.95, 900),
            sample_plan("b", "theme-a", 0.93, 900),
            sample_plan("c", "theme-a", 0.91, 900),
            sample_plan("d", "theme-b", 0.90, 900),
        ];

        let day = Planner::build_day(&card, plans);
        let themes: Vec<_> = day
            .scheduled
            .iter()
            .map(|p| p.theme_tags.first().cloned().unwrap_or_default())
            .collect();
        let mut streak = 1;
        for window in themes.windows(2) {
            if window[0] == window[1] {
                streak += 1;
                assert!(streak <= 2, "theme streak exceeded configured limit");
            } else {
                streak = 1;
            }
        }
    }

    #[test]
    fn planner_deduplicates_urls_and_titles() {
        let card = sample_card();
        let plans = vec![
            sample_plan("x", "theme-a", 0.9, 800),
            sample_plan("x", "theme-b", 0.8, 800),
            PlanItem {
                title: "SAME TITLE".to_string(),
                ..sample_plan("y", "theme-b", 0.95, 800)
            },
            PlanItem {
                title: "same title".to_string(),
                ..sample_plan("z", "theme-c", 0.92, 800)
            },
        ];

        let day = Planner::build_day(&card, plans);
        assert_eq!(day.scheduled.len() + day.reserves.len(), 2);
    }

    fn sample_card() -> OwnerCard {
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
                buffer_target_minutes: 120,
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

    fn sample_plan(id: &str, theme: &str, score: f32, duration_sec: u32) -> PlanItem {
        PlanItem {
            plan_id: id.to_string(),
            source_url: format!("https://example.com/{id}"),
            source_domain: "example.com".to_string(),
            discovered_at: Utc::now(),
            title: format!("title-{id}"),
            duration_sec,
            theme_tags: vec![theme.to_string()],
            visual_features: vec![],
            quality_signals: vec![],
            selection_reason: "test".to_string(),
            policy_match_score: score,
            state: PlanState::Candidate,
        }
    }
}
