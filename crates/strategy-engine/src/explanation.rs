//! Human-readable explanation of a decision, for the dashboard and reports.

use crate::alpha_score::ScoredAsset;
use crate::regime::MarketRegime;
use common::{OrderIntent, TargetPosition};
use market_data::FearGreedSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyExplanation {
    pub regime: String,
    pub headline: String,
    pub top_scores: Vec<(String, f64)>,
    pub target_summary: Vec<(String, String)>,
    pub order_count: usize,
    pub fear_greed: Option<u32>,
}

/// Assemble an explanation from the decision components.
pub fn build(
    regime: MarketRegime,
    scored: &[ScoredAsset],
    targets: &[TargetPosition],
    orders: &[OrderIntent],
    fear_greed: Option<&FearGreedSnapshot>,
) -> StrategyExplanation {
    let top_scores = scored
        .iter()
        .take(5)
        .map(|s| (s.symbol.clone(), (s.score * 1000.0).round() / 1000.0))
        .collect::<Vec<_>>();

    let target_summary = targets
        .iter()
        .map(|t| (t.symbol.clone(), format!("{}%", t.weight_pct)))
        .collect::<Vec<_>>();

    let headline = match regime {
        MarketRegime::Breakout => "Breakout regime — leaning into strength.",
        MarketRegime::RiskOn => "Risk-on — constructive allocation.",
        MarketRegime::Chop => "Choppy — reduced exposure, holding reserve.",
        MarketRegime::RiskOff => "Risk-off — defensive, mostly stables.",
    }
    .to_string();

    StrategyExplanation {
        regime: regime.as_str().to_string(),
        headline,
        top_scores,
        target_summary,
        order_count: orders.len(),
        fear_greed: fear_greed.map(|f| f.value),
    }
}
