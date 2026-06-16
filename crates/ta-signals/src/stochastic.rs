//! Stochastic oscillator (%K / %D).

use crate::candle::Candle;
use crate::sma::sma;

/// Stochastic oscillator over `period` with a `%D` smoothing of `smooth`
/// (classically 14, 3). Returns `(k, d)` aligned with `candles`.
///
/// `%K = 100 * (close - lowest_low) / (highest_high - lowest_low)` over the
/// lookback window; `%D = SMA(%K, smooth)`. A flat window (`high == low`) yields
/// `%K = 50` (neutral) rather than a division by zero.
///
/// # Panics
/// Panics if `period == 0` or `smooth == 0`.
pub fn stochastic(candles: &[Candle], period: usize, smooth: usize) -> (Vec<f64>, Vec<f64>) {
    assert!(period > 0 && smooth > 0, "stochastic periods must be > 0");
    let n = candles.len();
    let mut k = vec![f64::NAN; n];
    if n < period {
        return (k, vec![f64::NAN; n]);
    }
    for i in (period - 1)..n {
        let window = &candles[i + 1 - period..=i];
        let hh = window.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let ll = window.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        let denom = hh - ll;
        k[i] = if denom == 0.0 {
            50.0
        } else {
            100.0 * (candles[i].close - ll) / denom
        };
    }

    // %D = SMA(smooth) over the valid portion of %K.
    let mut d = vec![f64::NAN; n];
    if let Some(start) = k.iter().position(|v| !v.is_nan()) {
        let sd = sma(&k[start..], smooth);
        for (offset, v) in sd.into_iter().enumerate() {
            d[start + offset] = v;
        }
    }
    (k, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(h: f64, l: f64, c: f64) -> Candle {
        Candle::new((h + l) / 2.0, h, l, c, 1.0)
    }

    #[test]
    fn close_at_top_of_range_is_100() {
        // Range [10,20] for the whole window, last close at the high (20).
        let mut candles: Vec<Candle> = (0..14).map(|_| candle(20.0, 10.0, 15.0)).collect();
        candles.push(candle(20.0, 10.0, 20.0));
        let (k, _d) = stochastic(&candles, 14, 3);
        assert!((k[14] - 100.0).abs() < 1e-9);
    }

    #[test]
    fn close_at_bottom_of_range_is_0() {
        let mut candles: Vec<Candle> = (0..14).map(|_| candle(20.0, 10.0, 15.0)).collect();
        candles.push(candle(20.0, 10.0, 10.0));
        let (k, _d) = stochastic(&candles, 14, 3);
        assert!(k[14].abs() < 1e-9);
    }
}
