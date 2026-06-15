//! Execution-quality feature: how cleanly we expect to trade this asset, based
//! on liquidity depth versus a nominal clip size.

use crate::normalization::clamp01;
use common::decimal::to_f64;
use market_data::AssetMarketState;

/// Nominal clip used to gauge expected impact, in USD.
const NOMINAL_CLIP_USD: f64 = 2_000.0;

pub fn score(state: &AssetMarketState) -> f64 {
    let liq = state.liquidity_usd.map(to_f64).unwrap_or(0.0);
    if liq <= 0.0 {
        return 0.0;
    }
    let impact = NOMINAL_CLIP_USD / liq; // fraction of pool consumed
    clamp01(1.0 - impact * 20.0)
}
