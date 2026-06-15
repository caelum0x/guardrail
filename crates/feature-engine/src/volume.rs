//! Volume feature: rewards assets whose 24h volume is meaningful relative to
//! market cap (turnover), a proxy for participation.

use crate::normalization::min_max;
use common::decimal::to_f64;
use market_data::AssetMarketState;

pub fn score(state: &AssetMarketState) -> f64 {
    let vol = to_f64(state.volume_24h_usd);
    let mcap = state.market_cap_usd.map(to_f64).unwrap_or(0.0);
    if mcap <= 0.0 {
        // No market cap: fall back to absolute volume on a log scale.
        return min_max((vol.max(1.0)).log10(), 4.0, 8.0);
    }
    let turnover = vol / mcap;
    min_max(turnover, 0.0, 0.5)
}
