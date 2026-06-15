//! Risk-penalty feature: a 0..1 penalty (higher = worse) from security signals.

use market_data::AssetMarketState;

pub fn penalty(state: &AssetMarketState) -> f64 {
    let mut penalty = 0.0;
    // Low safety score contributes proportionally.
    penalty += (100u32.saturating_sub(state.safety_score)) as f64 / 100.0 * 0.5;
    // Each flag adds a fixed penalty.
    penalty += state.security_flags.len() as f64 * 0.25;
    penalty.clamp(0.0, 1.0)
}
