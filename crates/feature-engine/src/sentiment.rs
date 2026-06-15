//! Sentiment feature: derived from the market-wide Fear & Greed index.
//! A single market value applies to every risk asset this cycle.

use crate::normalization::clamp01;

/// Map the 0..100 Fear & Greed value into a 0..1 tailwind score.
/// Greed is a tailwind for momentum; extreme greed is slightly discounted.
pub fn score_from_fear_greed(value: u32) -> f64 {
    let v = value as f64;
    if v <= 75.0 {
        clamp01(v / 75.0)
    } else {
        // Taper extreme greed (overheated market).
        clamp01(1.0 - (v - 75.0) / 50.0)
    }
}
