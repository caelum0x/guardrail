//! Exponential Moving Average (EMA).

/// Compute the EMA over `period` samples, seeded with the SMA of the first
/// `period` values (the conventional seeding). Aligned with `data`; the first
/// `period - 1` entries are `NaN`.
///
/// # Panics
/// Panics if `period == 0`.
pub fn ema(data: &[f64], period: usize) -> Vec<f64> {
    assert!(period > 0, "ema period must be > 0");
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if n < period {
        return out;
    }
    let k = 2.0 / (period as f64 + 1.0);
    let seed = data[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = seed;
    for i in period..n {
        out[i] = data[i] * k + out[i - 1] * (1.0 - k);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ema_seed_is_sma_of_first_window() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0];
        let r = ema(&data, 3);
        assert!(r[0].is_nan() && r[1].is_nan());
        assert!((r[2] - 2.0).abs() < 1e-12); // SMA(1,2,3)
        // next: 4*0.5 + 2*0.5 = 3.0 ; then 5*0.5 + 3*0.5 = 4.0
        assert!((r[3] - 3.0).abs() < 1e-12);
        assert!((r[4] - 4.0).abs() < 1e-12);
    }

    #[test]
    fn ema_reacts_faster_than_sma() {
        let data = [1.0, 1.0, 1.0, 10.0];
        let r = ema(&data, 2);
        // last EMA pulled strongly toward 10
        assert!(r[3] > 5.0);
    }
}
