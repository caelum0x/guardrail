//! Simple Moving Average (SMA).

/// Compute the Simple Moving Average over `period` samples.
///
/// Returns a `Vec<f64>` aligned with `data`. The first `period - 1` entries are
/// `NaN` (warmup). Uses an O(n) sliding-window sum.
///
/// # Panics
/// Panics if `period == 0`.
pub fn sma(data: &[f64], period: usize) -> Vec<f64> {
    assert!(period > 0, "sma period must be > 0");
    let mut out = vec![f64::NAN; data.len()];
    if data.len() < period {
        return out;
    }

    let mut window_sum: f64 = data[..period].iter().sum();
    out[period - 1] = window_sum / period as f64;

    for i in period..data.len() {
        window_sum += data[i] - data[i - period];
        out[i] = window_sum / period as f64;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sma_of_1_to_5_period_3() {
        // windows: [1,2,3]=2, [2,3,4]=3, [3,4,5]=4
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let r = sma(&data, 3);
        assert!(r[0].is_nan());
        assert!(r[1].is_nan());
        assert!((r[2] - 2.0).abs() < 1e-12);
        assert!((r[3] - 3.0).abs() < 1e-12);
        assert!((r[4] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn sma_full_window() {
        let data = [2.0, 4.0, 6.0, 8.0];
        let r = sma(&data, 4);
        assert!(r[0].is_nan() && r[1].is_nan() && r[2].is_nan());
        assert!((r[3] - 5.0).abs() < 1e-12);
    }

    #[test]
    fn sma_too_short_is_all_nan() {
        let data = [1.0, 2.0];
        let r = sma(&data, 5);
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|v| v.is_nan()));
    }
}
