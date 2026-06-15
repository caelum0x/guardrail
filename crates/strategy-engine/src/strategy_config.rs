//! Strategy configuration: feature-blend weights and gating thresholds.

use portfolio_optimizer::AllocationMethod;
use serde::{Deserialize, Serialize};

/// Weights applied to each normalized feature when computing the alpha score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureWeights {
    pub momentum: f64,
    pub volume: f64,
    pub volatility: f64,
    pub liquidity: f64,
    pub sentiment: f64,
    pub execution_quality: f64,
    pub risk_penalty: f64,
}

impl Default for FeatureWeights {
    fn default() -> Self {
        FeatureWeights {
            momentum: 0.30,
            volume: 0.15,
            volatility: 0.10,
            liquidity: 0.15,
            sentiment: 0.10,
            execution_quality: 0.20,
            risk_penalty: 1.0, // subtracted, scaled separately
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    #[serde(default)]
    pub weights: FeatureWeights,
    pub max_positions: u32,
    pub min_score_to_enter: f64,
    pub min_score_to_hold: f64,
    pub rebalance_threshold_pct: f64,
    /// Floor on the stable reserve weight the allocator leaves untouched.
    #[serde(default = "default_reserve")]
    pub target_stable_reserve_pct: f64,
    /// Hard cap on any single position weight (percent of NAV). Kept at or
    /// below the risk policy's `max_position_pct` so targets are not rejected.
    #[serde(default = "default_max_position")]
    pub max_position_weight_pct: f64,
    /// Protective stop-loss: force-exit a position once its unrealized loss
    /// reaches this percent of average cost.
    #[serde(default = "default_stop_loss")]
    pub stop_loss_pct: f64,
    /// Protective take-profit: force-exit a position once its unrealized gain
    /// reaches this percent of average cost.
    #[serde(default = "default_take_profit")]
    pub take_profit_pct: f64,
    /// Method used to translate selected assets' scores into per-name weights.
    /// Defaults to score-proportional, preserving the legacy allocator behavior.
    #[serde(default = "default_allocation_method")]
    pub allocation_method: AllocationMethod,
}

fn default_reserve() -> f64 {
    15.0
}

fn default_max_position() -> f64 {
    17.0
}

fn default_stop_loss() -> f64 {
    12.0
}

fn default_take_profit() -> f64 {
    25.0
}

fn default_allocation_method() -> AllocationMethod {
    AllocationMethod::ScoreProportional
}

impl Default for StrategyConfig {
    fn default() -> Self {
        StrategyConfig {
            weights: FeatureWeights::default(),
            max_positions: 5,
            min_score_to_enter: 0.65,
            min_score_to_hold: 0.50,
            rebalance_threshold_pct: 3.0,
            target_stable_reserve_pct: 15.0,
            max_position_weight_pct: 17.0,
            stop_loss_pct: 12.0,
            take_profit_pct: 25.0,
            allocation_method: AllocationMethod::ScoreProportional,
        }
    }
}
