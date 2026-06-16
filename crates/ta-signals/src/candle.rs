//! OHLCV candle type and helpers for extracting aligned price series.

use serde::{Deserialize, Serialize};

/// A single OHLCV price bar.
///
/// All fields are `f64`. The struct is `Copy` so candle slices can be
/// iterated cheaply by value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Candle {
    /// Construct a candle from its components.
    pub fn new(open: f64, high: f64, low: f64, close: f64, volume: f64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Typical price = (high + low + close) / 3. Used by VWAP and CCI-style
    /// indicators.
    #[inline]
    pub fn typical_price(&self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }
}

/// Extract the close series from candles.
pub fn closes(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|c| c.close).collect()
}

/// Extract the high series from candles.
pub fn highs(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|c| c.high).collect()
}

/// Extract the low series from candles.
pub fn lows(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|c| c.low).collect()
}

/// Extract the volume series from candles.
pub fn volumes(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|c| c.volume).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typical_price_is_average_of_hlc() {
        let c = Candle::new(10.0, 12.0, 8.0, 11.0, 100.0);
        // (12 + 8 + 11) / 3 = 31/3
        assert!((c.typical_price() - 31.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn series_extractors_align() {
        let cs = vec![
            Candle::new(1.0, 2.0, 0.5, 1.5, 10.0),
            Candle::new(1.5, 2.5, 1.0, 2.0, 20.0),
        ];
        assert_eq!(closes(&cs), vec![1.5, 2.0]);
        assert_eq!(highs(&cs), vec![2.0, 2.5]);
        assert_eq!(lows(&cs), vec![0.5, 1.0]);
        assert_eq!(volumes(&cs), vec![10.0, 20.0]);
    }
}
