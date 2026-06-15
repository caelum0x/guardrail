//! Classic technical indicators over price / candle series.
//!
//! All indicator functions are **pure** (no I/O) and operate on `&[f64]`
//! inputs to stay dependency-light and fast. Callers holding
//! `rust_decimal::Decimal` data (such as `cmc-client`'s `Candle`) can convert
//! with the helpers in [`convert`].
//!
//! # Output contract
//!
//! Every indicator returns a `Vec<f64>` (or a struct of such vectors) that is
//! **aligned to the input** (same length) and **free of `NaN`/`None`**.
//! Warm-up positions before the first fully-formed value repeat the earliest
//! computable value so downstream consumers never encounter gaps. When there is
//! insufficient data (or invalid parameters such as a zero period), an **empty**
//! vector / struct-of-empty-vectors is returned.

#![forbid(unsafe_code)]

pub mod atr;
pub mod bollinger;
pub mod convert;
pub mod macd;
pub mod moving_average;
pub mod rsi;

pub use atr::atr;
pub use bollinger::{bollinger, Bollinger};
pub use convert::{decimal_to_f64, decimals_to_f64};
pub use macd::{macd, Macd};
pub use moving_average::{ema, sma};
pub use rsi::rsi;

#[cfg(test)]
mod tests {
    use super::*;

    /// Floating-point comparison helper for tests.
    fn approx(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn sma_of_constant_series_equals_constant() {
        let values = vec![5.0; 10];
        let result = sma(&values, 3);
        assert_eq!(result.len(), values.len());
        for v in result {
            assert!(approx(v, 5.0, 1e-9));
        }
    }

    #[test]
    fn ema_of_constant_series_equals_constant() {
        let values = vec![42.0; 20];
        let result = ema(&values, 5);
        assert_eq!(result.len(), values.len());
        for v in result {
            assert!(approx(v, 42.0, 1e-9), "expected 42.0, got {v}");
        }
    }

    #[test]
    fn sma_rolling_window_is_correct() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&values, 3);
        // index 2 -> mean(1,2,3) = 2.0
        assert!(approx(result[2], 2.0, 1e-9));
        // index 3 -> mean(2,3,4) = 3.0
        assert!(approx(result[3], 3.0, 1e-9));
        // index 4 -> mean(3,4,5) = 4.0
        assert!(approx(result[4], 4.0, 1e-9));
    }

    #[test]
    fn insufficient_data_returns_empty() {
        assert!(sma(&[1.0, 2.0], 5).is_empty());
        assert!(ema(&[1.0, 2.0], 5).is_empty());
        assert!(rsi(&[1.0, 2.0], 5).is_empty());
        assert!(sma(&[1.0, 2.0, 3.0], 0).is_empty());
    }

    #[test]
    fn rsi_rising_series_approaches_100() {
        // Strictly increasing series: no losses -> RSI should be 100.
        let values: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let result = rsi(&values, 14);
        assert_eq!(result.len(), values.len());
        let last = *result.last().unwrap();
        assert!(last > 99.0, "expected RSI near 100, got {last}");
    }

    #[test]
    fn rsi_falling_series_approaches_0() {
        let values: Vec<f64> = (0..30).map(|i| 100.0 - i as f64).collect();
        let result = rsi(&values, 14);
        let last = *result.last().unwrap();
        assert!(last < 1.0, "expected RSI near 0, got {last}");
    }

    #[test]
    fn rsi_is_bounded() {
        let values = vec![
            10.0, 11.0, 9.0, 12.0, 8.0, 13.0, 7.0, 14.0, 6.0, 15.0, 5.0, 16.0,
        ];
        let result = rsi(&values, 3);
        assert_eq!(result.len(), values.len());
        for v in result {
            assert!((0.0..=100.0).contains(&v), "RSI out of bounds: {v}");
        }
    }

    #[test]
    fn macd_lengths_are_aligned() {
        let values: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin() + 10.0).collect();
        let out = macd(&values, 12, 26, 9);
        assert_eq!(out.macd.len(), values.len());
        assert_eq!(out.signal.len(), values.len());
        assert_eq!(out.histogram.len(), values.len());
        // histogram == macd - signal
        for i in 0..values.len() {
            assert!(approx(out.histogram[i], out.macd[i] - out.signal[i], 1e-9));
        }
    }

    #[test]
    fn macd_invalid_params_returns_empty() {
        let values: Vec<f64> = (0..50).map(|i| i as f64).collect();
        // fast >= slow
        let out = macd(&values, 26, 12, 9);
        assert!(out.macd.is_empty());
        assert!(out.signal.is_empty());
        assert!(out.histogram.is_empty());
    }

    #[test]
    fn bollinger_bands_ordering_and_lengths() {
        let values: Vec<f64> = (0..50).map(|i| (i as f64).sin() * 5.0 + 20.0).collect();
        let bb = bollinger(&values, 20, 2.0);
        assert_eq!(bb.mid.len(), values.len());
        assert_eq!(bb.upper.len(), values.len());
        assert_eq!(bb.lower.len(), values.len());
        for i in 0..values.len() {
            assert!(bb.upper[i] >= bb.mid[i], "upper < mid at {i}");
            assert!(bb.mid[i] >= bb.lower[i], "mid < lower at {i}");
        }
    }

    #[test]
    fn bollinger_constant_series_has_zero_width() {
        let values = vec![7.0; 30];
        let bb = bollinger(&values, 10, 2.0);
        for i in 0..values.len() {
            assert!(approx(bb.mid[i], 7.0, 1e-9));
            assert!(approx(bb.upper[i], 7.0, 1e-9));
            assert!(approx(bb.lower[i], 7.0, 1e-9));
        }
    }

    #[test]
    fn atr_lengths_and_positive() {
        let high: Vec<f64> = (0..30).map(|i| 10.0 + i as f64 + 1.0).collect();
        let low: Vec<f64> = (0..30).map(|i| 10.0 + i as f64 - 1.0).collect();
        let close: Vec<f64> = (0..30).map(|i| 10.0 + i as f64).collect();
        let result = atr(&high, &low, &close, 14);
        assert_eq!(result.len(), high.len());
        for v in result {
            assert!(v >= 0.0, "ATR should be non-negative, got {v}");
        }
    }

    #[test]
    fn atr_mismatched_lengths_returns_empty() {
        let high = vec![1.0, 2.0, 3.0];
        let low = vec![1.0, 2.0];
        let close = vec![1.0, 2.0, 3.0];
        assert!(atr(&high, &low, &close, 2).is_empty());
    }

    #[test]
    fn decimals_conversion_roundtrip() {
        use rust_decimal::Decimal;
        let decs = vec![Decimal::new(125, 1), Decimal::new(250, 1)]; // 12.5, 25.0
        let fs = decimals_to_f64(&decs);
        assert_eq!(fs.len(), 2);
        assert!(approx(fs[0], 12.5, 1e-9));
        assert!(approx(fs[1], 25.0, 1e-9));
        assert!(approx(
            decimal_to_f64(Decimal::new(333, 2)).unwrap(),
            3.33,
            1e-9
        ));
    }
}
