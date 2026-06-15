//! Liquidity helpers used by the risk engine's liquidity check.

use crate::snapshot::AssetMarketState;
use common::Decimal;

/// Does the asset clear a minimum on-chain liquidity floor?
pub fn meets_liquidity_floor(state: &AssetMarketState, floor_usd: Decimal) -> bool {
    state.liquidity_usd.map(|l| l >= floor_usd).unwrap_or(false)
}

/// Estimated fraction of liquidity a trade of `notional_usd` would consume.
pub fn liquidity_consumption(state: &AssetMarketState, notional_usd: Decimal) -> Option<Decimal> {
    let liq = state.liquidity_usd?;
    if liq.is_zero() {
        return None;
    }
    Some(notional_usd / liq * Decimal::from(100))
}
