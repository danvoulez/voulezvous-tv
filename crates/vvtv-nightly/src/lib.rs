use vvtv_types::{OwnerCard, PipelineMetrics};

pub struct Nightly;

impl Nightly {
    #[must_use]
    pub fn tune(owner_card: &OwnerCard, metrics: &PipelineMetrics) -> String {
        if !owner_card.autotune_policy.enabled {
            return "autotune-disabled".to_string();
        }

        if metrics.fallback_rate > 0.3 {
            return format!(
                "increase-reserve-pool-by-{:.1}%",
                owner_card.autotune_policy.max_daily_adjustment_pct.min(3.0)
            );
        }
        if metrics.qa_pass_rate < 0.8 {
            return "tighten-quality-thresholds-within-daily-limit".to_string();
        }
        "no-change".to_string()
    }
}
