//! Candle helpers shared by the feature engine and backtester.

pub use cmc_client::Candle;
use common::Decimal;

/// Simple close-to-close return over the candle series, in percent.
pub fn series_return_pct(candles: &[Candle]) -> Option<Decimal> {
    let first = candles.first()?;
    let last = candles.last()?;
    if first.close.is_zero() {
        return None;
    }
    Some((last.close - first.close) / first.close * Decimal::from(100))
}

/// Mean candle volume.
pub fn mean_volume(candles: &[Candle]) -> Option<Decimal> {
    if candles.is_empty() {
        return None;
    }
    let total: Decimal = candles.iter().map(|c| c.volume).sum();
    Some(total / Decimal::from(candles.len() as i64))
}
