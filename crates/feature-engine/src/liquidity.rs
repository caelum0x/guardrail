//! Liquidity feature: deeper on-chain liquidity scores higher (log-scaled).

use crate::normalization::min_max;
use common::decimal::to_f64;
use market_data::AssetMarketState;

pub fn score(state: &AssetMarketState) -> f64 {
    let liq = state.liquidity_usd.map(to_f64).unwrap_or(0.0);
    min_max((liq.max(1.0)).log10(), 5.0, 8.0) // $100k .. $100M
}
