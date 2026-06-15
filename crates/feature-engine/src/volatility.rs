//! Volatility feature: moderate volatility scores best. Too flat means no
//! opportunity; too wild means execution risk.

use crate::normalization::clamp01;
use common::decimal::to_f64;
use market_data::AssetMarketState;

pub fn score(state: &AssetMarketState) -> f64 {
    let vol = state.volatility_1h.map(to_f64).unwrap_or(2.0);
    // Triangular preference peaking around 3% hourly range.
    let ideal = 3.0;
    let distance = (vol - ideal).abs();
    clamp01(1.0 - distance / 6.0)
}
