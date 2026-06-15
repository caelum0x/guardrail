//! PnL aggregation across the portfolio.

use crate::portfolio_state::PortfolioState;
use common::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlSummary {
    pub realized_usd: Decimal,
    pub unrealized_usd: Decimal,
    pub total_usd: Decimal,
}

pub fn summarize(state: &PortfolioState) -> PnlSummary {
    let unrealized: Decimal = state.holdings.iter().map(|h| h.unrealized_pnl_usd()).sum();
    PnlSummary {
        realized_usd: state.realized_pnl_usd,
        unrealized_usd: unrealized,
        total_usd: state.realized_pnl_usd + unrealized,
    }
}
