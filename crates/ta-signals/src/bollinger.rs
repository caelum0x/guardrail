//! Bollinger Bands.

use crate::sma::sma;

/// Bollinger Bands over `period` with `mult` standard deviations (classically
/// 20, 2.0). Returns `(upper, middle, lower)` aligned with `data`; the first
/// `period - 1` entries are `NaN`. Uses the population standard deviation over
/// each window (the conventional Bollinger definition).
///
/// # Panics
/// Panics if `period == 0`.
pub fn bollinger(data: &[f64], period: usize, mult: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    assert!(period > 0, "bollinger period must be > 0");
    let n = data.len();
    let middle = sma(data, period);
    let mut upper = vec![f64::NAN; n];
    let mut lower = vec![f64::NAN; n];
    if n < period {
        return (upper, middle, lower);
    }
    for i in (period - 1)..n {
        let mean = middle[i];
        let window = &data[i + 1 - period..=i];
        let var = window.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / period as f64;
        let sd = var.sqrt();
        upper[i] = mean + mult * sd;
        lower[i] = mean - mult * sd;
    }
    (upper, middle, lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_series_has_zero_width_bands() {
        let data = [5.0; 10];
        let (u, m, l) = bollinger(&data, 5, 2.0);
        assert!((m[9] - 5.0).abs() < 1e-12);
        assert!((u[9] - 5.0).abs() < 1e-12);
        assert!((l[9] - 5.0).abs() < 1e-12);
    }

    #[test]
    fn bands_straddle_the_mean() {
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let (u, m, l) = bollinger(&data, 3, 2.0);
        let i = 5;
        assert!(u[i] > m[i] && m[i] > l[i]);
        // symmetric around mean
        assert!(((u[i] - m[i]) - (m[i] - l[i])).abs() < 1e-12);
    }
}
