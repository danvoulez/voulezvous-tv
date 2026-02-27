use std::collections::HashSet;

use vvtv_types::{OwnerCard, PlanItem, PlanState, PlannedDay};

pub struct Planner;

impl Planner {
    #[must_use]
    pub fn build_day(owner_card: &OwnerCard, mut plans: Vec<PlanItem>) -> PlannedDay {
        let mut seen_themes: HashSet<String> = HashSet::new();
        let mut scheduled = Vec::new();
        let mut reserves = Vec::new();

        plans.sort_by(|a, b| b.policy_match_score.total_cmp(&a.policy_match_score));

        for mut plan in plans {
            let primary_theme = plan
                .theme_tags
                .first()
                .cloned()
                .unwrap_or_else(|| "generic".to_string());
            if seen_themes.len()
                < usize::from(owner_card.editorial_profile.min_unique_themes_per_block)
                && !seen_themes.contains(&primary_theme)
            {
                seen_themes.insert(primary_theme);
                plan.state = PlanState::Scheduled;
                scheduled.push(plan);
            } else {
                plan.state = PlanState::Reserved;
                reserves.push(plan);
            }
        }

        PlannedDay {
            scheduled,
            reserves,
        }
    }
}
