//! The compact set of inputs the strategy's regime classifier consumes.

use crate::snapshot::MarketSnapshot;
use common::Decimal;

#[derive(Debug, Clone)]
pub struct RegimeInputs {
    pub fear_greed: u32,
    pub breadth_pct: Decimal,
    pub btc_dominance_pct: Decimal,
    pub median_24h_return: Decimal,
}

impl RegimeInputs {
    /// Derive regime inputs from a snapshot.
    pub fn from_snapshot(snap: &MarketSnapshot) -> Self {
        let non_stable: Vec<&_> = snap
            .assets
            .iter()
            .filter(|a| !a.asset.category.is_stable())
            .collect();

        let advancing = non_stable
            .iter()
            .filter(|a| a.ret_24h.map(|r| r > Decimal::ZERO).unwrap_or(false))
            .count();

        let breadth_pct = if non_stable.is_empty() {
            Decimal::ZERO
        } else {
            Decimal::from(advancing as i64) / Decimal::from(non_stable.len() as i64)
                * Decimal::from(100)
        };

        let mut returns: Vec<Decimal> = non_stable.iter().filter_map(|a| a.ret_24h).collect();
        returns.sort();
        let median = if returns.is_empty() {
            Decimal::ZERO
        } else {
            returns[returns.len() / 2]
        };

        RegimeInputs {
            fear_greed: snap.fear_greed.as_ref().map(|f| f.value).unwrap_or(50),
            breadth_pct,
            btc_dominance_pct: snap
                .global_market
                .as_ref()
                .map(|g| g.btc_dominance_pct)
                .unwrap_or_default(),
            median_24h_return: median,
        }
    }
}
