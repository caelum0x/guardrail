//! Strategy engine: decides the *target* portfolio and the orders needed to
//! reach it. It has no execution authority and never signs or calls TWAK.
//!
//! Flow: snapshot -> features -> regime -> alpha scores -> target weights ->
//! rebalance orders -> explanation.

pub mod allocator;
pub mod alpha_score;
pub mod daily_trade;
pub mod exits;
pub mod explanation;
pub mod rebalance;
pub mod regime;
pub mod strategy_config;
pub mod target_portfolio;

pub use explanation::StrategyExplanation;
pub use regime::MarketRegime;
pub use strategy_config::StrategyConfig;
pub use target_portfolio::{CurrentAllocation, StrategyDecision};

use common::time::now_ms;
use feature_engine::FeatureEngine;
use market_data::{MarketSnapshot, RegimeInputs};

/// The top-level strategy. Wires features, regime, scoring, and rebalancing.
pub struct StrategyEngine {
    config: StrategyConfig,
    features: FeatureEngine,
}

impl StrategyEngine {
    pub fn new(config: StrategyConfig) -> Self {
        StrategyEngine {
            config,
            features: FeatureEngine::new(),
        }
    }

    /// Produce a full decision for the current snapshot and portfolio state.
    pub fn decide(
        &self,
        snap: &MarketSnapshot,
        current: &CurrentAllocation,
        nav_usd: common::Decimal,
    ) -> StrategyDecision {
        let regime = regime::classify(&RegimeInputs::from_snapshot(snap));
        let features = self.features.compute(snap);
        let scored = alpha_score::compute(&features, &self.config);
        let targets = allocator::build_targets(&scored, regime, &self.config);
        let orders = rebalance::compute_orders(&targets, current, nav_usd, &self.config);
        let explanation =
            explanation::build(regime, &scored, &targets, &orders, snap.fear_greed.as_ref());

        StrategyDecision {
            timestamp_ms: now_ms(),
            regime,
            target_positions: targets,
            proposed_orders: orders,
            explanation,
        }
    }

    pub fn config(&self) -> &StrategyConfig {
        &self.config
    }
}
