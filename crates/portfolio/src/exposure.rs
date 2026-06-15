//! Exposure aggregation by asset category, for category-level risk views.

use crate::portfolio_state::PortfolioState;
use common::Decimal;
use std::collections::HashMap;

/// Largest single non-reserve position weight, in percent of NAV.
pub fn max_position_pct(state: &PortfolioState) -> Decimal {
    state
        .risk_weights_pct()
        .values()
        .copied()
        .max()
        .unwrap_or(Decimal::ZERO)
}

/// Total non-reserve (at-risk) exposure, in percent of NAV.
pub fn risk_exposure_pct(state: &PortfolioState) -> Decimal {
    state.risk_weights_pct().values().copied().sum()
}

/// Per-symbol weights as a plain map (clone of the risk weights).
pub fn weights(state: &PortfolioState) -> HashMap<String, Decimal> {
    state.risk_weights_pct()
}
