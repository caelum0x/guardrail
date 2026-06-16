//! Average True Range (ATR), Wilder's smoothing.

use crate::candle::Candle;

/// True Range for bar `i` given the previous close.
fn true_range(c: &Candle, prev_close: f64) -> f64 {
    let hl = c.high - c.low;
    let hc = (c.high - prev_close).abs();
    let lc = (c.low - prev_close).abs();
    hl.max(hc).max(lc)
}

/// Wilder's ATR over `period` (classically 14). Aligned with `candles`; the
/// first ATR value lands at index `period`. Entries before that are `NaN`.
///
/// # Panics
/// Panics if `period == 0`.
pub fn atr(candles: &[Candle], period: usize) -> Vec<f64> {
    assert!(period > 0, "atr period must be > 0");
    let n = candles.len();
    let mut out = vec![f64::NAN; n];
    if n <= period {
        return out;
    }

    // True range series (tr[0] uses high-low, no prior close).
    let mut tr = vec![0.0; n];
    tr[0] = candles[0].high - candles[0].low;
    for i in 1..n {
        tr[i] = true_range(&candles[i], candles[i - 1].close);
    }

    let p = period as f64;
    // Seed: simple average of the first `period` true ranges (tr[1..=period]).
    let mut atr = tr[1..=period].iter().sum::<f64>() / p;
    out[period] = atr;
    for i in (period + 1)..n {
        atr = (atr * (p - 1.0) + tr[i]) / p;
        out[i] = atr;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(h: f64, l: f64, c: f64) -> Candle {
        Candle::new((h + l) / 2.0, h, l, c, 1.0)
    }

    #[test]
    fn constant_range_atr_equals_range() {
        // Every bar has a true range of exactly 2.0 -> ATR converges to 2.0.
        let candles: Vec<Candle> = (0..20).map(|_| candle(11.0, 9.0, 10.0)).collect();
        let r = atr(&candles, 14);
        assert!(r[14].is_finite());
        assert!((r[14] - 2.0).abs() < 1e-9);
        assert!((*r.last().unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn warmup_is_nan() {
        let candles: Vec<Candle> = (0..16).map(|_| candle(11.0, 9.0, 10.0)).collect();
        let r = atr(&candles, 14);
        for v in r.iter().take(14) {
            assert!(v.is_nan());
        }
        assert!(r[14].is_finite());
    }
}
