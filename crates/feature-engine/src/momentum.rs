//! Momentum feature: blends 1h and 24h returns into a 0..1 score.

use crate::normalization::sigmoid;
use common::decimal::to_f64;
use market_data::AssetMarketState;

pub fn score(state: &AssetMarketState) -> f64 {
    let r1h = state.ret_1h.map(to_f64).unwrap_or(0.0);
    let r24h = state.ret_24h.map(to_f64).unwrap_or(0.0);
    // Weight recent momentum more, scale into the sigmoid's sensitive range.
    let blended = (0.6 * r1h + 0.4 * r24h) / 5.0;
    sigmoid(blended)
}
