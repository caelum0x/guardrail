//! Relative Strength Index (RSI), Wilder's smoothing.

/// Compute Wilder's RSI over `period` (classically 14). Aligned with `data`;
/// entries `[0..=period-1]`... actually `[0..period]` are `NaN` (the first RSI
/// value lands at index `period`, needing `period` price changes).
///
/// # Panics
/// Panics if `period == 0`.
pub fn rsi(data: &[f64], period: usize) -> Vec<f64> {
    assert!(period > 0, "rsi period must be > 0");
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n <= period {
        return out;
    }

    let (mut gain, mut loss) = (0.0, 0.0);
    for i in 1..=period {
        let change = data[i] - data[i - 1];
        if change >= 0.0 {
            gain += change;
        } else {
            loss -= change;
        }
    }
    let p = period as f64;
    let mut avg_gain = gain / p;
    let mut avg_loss = loss / p;
    out[period] = rsi_value(avg_gain, avg_loss);

    for i in (period + 1)..n {
        let change = data[i] - data[i - 1];
        let (g, l) = if change >= 0.0 { (change, 0.0) } else { (0.0, -change) };
        avg_gain = (avg_gain * (p - 1.0) + g) / p;
        avg_loss = (avg_loss * (p - 1.0) + l) / p;
        out[i] = rsi_value(avg_gain, avg_loss);
    }
    out
}

fn rsi_value(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        return 100.0;
    }
    let rs = avg_gain / avg_loss;
    100.0 - 100.0 / (1.0 + rs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_of_monotonic_rise_is_100() {
        let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
        let r = rsi(&data, 14);
        // strictly rising -> no losses -> RSI 100
        assert!((r[14] - 100.0).abs() < 1e-9);
        assert!((*r.last().unwrap() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn rsi_of_monotonic_fall_is_0() {
        let data: Vec<f64> = (1..=20).rev().map(|x| x as f64).collect();
        let r = rsi(&data, 14);
        assert!(r[14].abs() < 1e-9);
    }

    #[test]
    fn rsi_warmup_is_nan_and_range_is_bounded() {
        let data: Vec<f64> = (0..30).map(|i| (i as f64 * 0.7).sin() * 5.0 + 50.0).collect();
        let r = rsi(&data, 14);
        for v in r.iter().take(14) {
            assert!(v.is_nan());
        }
        for v in r.iter().skip(14) {
            assert!(*v >= 0.0 && *v <= 100.0);
        }
    }
}
