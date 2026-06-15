//! Reconciliation: compare the internal book against on-chain / TWAK balances
//! and surface drift. The agent alerts (and can halt) on a mismatch.

use crate::portfolio_state::PortfolioState;
use common::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconDrift {
    pub symbol: String,
    pub internal_qty: Decimal,
    pub external_qty: Decimal,
    pub abs_diff: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconReport {
    pub matched: bool,
    pub drifts: Vec<ReconDrift>,
}

/// Reconcile internal holdings against an external balance map (symbol -> qty).
/// `tolerance` is the absolute quantity difference allowed per symbol.
pub fn reconcile(
    state: &PortfolioState,
    external: &HashMap<String, Decimal>,
    tolerance: Decimal,
) -> ReconReport {
    let mut drifts = Vec::new();

    // Union of symbols on both sides.
    let mut symbols: Vec<String> = state.holdings.iter().map(|h| h.symbol.clone()).collect();
    for k in external.keys() {
        if !symbols.contains(k) {
            symbols.push(k.clone());
        }
    }

    for symbol in symbols {
        let internal = state
            .get(&symbol)
            .map(|h| h.quantity)
            .unwrap_or(Decimal::ZERO);
        let ext = external.get(&symbol).copied().unwrap_or(Decimal::ZERO);
        let diff = (internal - ext).abs();
        if diff > tolerance {
            drifts.push(ReconDrift {
                symbol,
                internal_qty: internal,
                external_qty: ext,
                abs_diff: diff,
            });
        }
    }

    ReconReport {
        matched: drifts.is_empty(),
        drifts,
    }
}
