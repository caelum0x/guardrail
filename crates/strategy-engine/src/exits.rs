//! Exit rules evaluated independently of rebalancing: hard stops that force a
//! position out regardless of target weights.

use crate::alpha_score::ScoredAsset;
use crate::strategy_config::StrategyConfig;
use common::decimal::to_f64;
use common::Decimal;

/// Should we force an exit of `symbol` given its latest score?
/// True when conviction falls below the hold threshold.
pub fn should_exit(scored: &[ScoredAsset], symbol: &str, cfg: &StrategyConfig) -> bool {
    match scored.iter().find(|s| s.symbol == symbol) {
        Some(s) => s.score < cfg.min_score_to_hold,
        None => true, // no longer scored at all -> exit
    }
}

/// Protective stop-loss: true when the unrealized loss relative to average cost
/// has reached `stop_pct` percent. Requires a positive average cost; otherwise
/// there is no basis to measure against and we never trigger.
pub fn stop_loss_hit(avg_cost_usd: Decimal, price_usd: Decimal, stop_pct: f64) -> bool {
    if avg_cost_usd <= Decimal::ZERO {
        return false;
    }
    let loss_pct = to_f64((avg_cost_usd - price_usd) / avg_cost_usd) * 100.0;
    loss_pct >= stop_pct
}

/// Protective take-profit: true when the unrealized gain relative to average
/// cost has reached `target_pct` percent. Requires a positive average cost.
pub fn take_profit_hit(avg_cost_usd: Decimal, price_usd: Decimal, target_pct: f64) -> bool {
    if avg_cost_usd <= Decimal::ZERO {
        return false;
    }
    let gain_pct = to_f64((price_usd - avg_cost_usd) / avg_cost_usd) * 100.0;
    gain_pct >= target_pct
}
