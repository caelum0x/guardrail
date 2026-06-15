//! The strategy's output type and the current-allocation input type.

use crate::explanation::StrategyExplanation;
use crate::regime::MarketRegime;
use common::{Decimal, OrderIntent, TargetPosition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A snapshot of what the portfolio currently holds, by weight and value.
/// Supplied by the portfolio accounting layer; the strategy treats it as read-only.
#[derive(Debug, Clone, Default)]
pub struct CurrentAllocation {
    /// symbol -> current weight as percent of NAV
    pub weights_pct: HashMap<String, Decimal>,
}

impl CurrentAllocation {
    pub fn new() -> Self {
        CurrentAllocation::default()
    }

    pub fn with_weight(mut self, symbol: impl Into<String>, weight_pct: Decimal) -> Self {
        self.weights_pct.insert(symbol.into(), weight_pct);
        self
    }

    pub fn weight(&self, symbol: &str) -> Decimal {
        self.weights_pct
            .get(symbol)
            .copied()
            .unwrap_or(Decimal::ZERO)
    }

    /// Non-reserve symbols currently held.
    pub fn held_symbols(&self) -> Vec<String> {
        self.weights_pct.keys().cloned().collect()
    }
}

/// The full strategy decision for one cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDecision {
    pub timestamp_ms: i64,
    pub regime: MarketRegime,
    pub target_positions: Vec<TargetPosition>,
    pub proposed_orders: Vec<OrderIntent>,
    pub explanation: StrategyExplanation,
}
