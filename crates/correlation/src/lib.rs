//! Correlation, covariance, and beta over return series (pure `f64`, no deps
//! beyond serde for the output types).
//!
//! All functions operate on equal-length slices (the shorter length is used
//! when they differ). A series with fewer than two points, or zero variance,
//! yields a defined fallback (0.0) rather than a NaN, so downstream risk math
//! never has to special-case it.
//!
//! ```
//! use correlation::pearson;
//! // Perfectly correlated.
//! assert!((pearson(&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0]) - 1.0).abs() < 1e-12);
//! // Perfectly anti-correlated.
//! assert!((pearson(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]) + 1.0).abs() < 1e-12);
//! ```

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

/// Sample covariance between two series (denominator `n - 1`).
pub fn covariance(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return 0.0;
    }
    let (mx, my) = (mean(&xs[..n]), mean(&ys[..n]));
    let acc: f64 = (0..n).map(|i| (xs[i] - mx) * (ys[i] - my)).sum();
    acc / (n as f64 - 1.0)
}

/// Sample variance of a series (denominator `n - 1`).
pub fn variance(xs: &[f64]) -> f64 {
    covariance(xs, xs)
}

/// Sample standard deviation.
pub fn stddev(xs: &[f64]) -> f64 {
    variance(xs).sqrt()
}

/// Pearson correlation coefficient in `[-1, 1]`. Returns 0.0 if either series
/// has zero variance or fewer than two points.
pub fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    let (sx, sy) = (stddev(xs), stddev(ys));
    if sx == 0.0 || sy == 0.0 {
        return 0.0;
    }
    (covariance(xs, ys) / (sx * sy)).clamp(-1.0, 1.0)
}

/// Beta of `asset` returns against `market` returns: `cov(a, m) / var(m)`.
/// Returns 0.0 when the market has zero variance.
pub fn beta(asset: &[f64], market: &[f64]) -> f64 {
    let vm = variance(market);
    if vm == 0.0 {
        return 0.0;
    }
    covariance(asset, market) / vm
}

/// Rolling Pearson correlation over a trailing `window`. Output is aligned with
/// the inputs; the first `window - 1` entries are `None` (warmup).
pub fn rolling_pearson(xs: &[f64], ys: &[f64], window: usize) -> Vec<Option<f64>> {
    let n = xs.len().min(ys.len());
    let mut out = vec![None; n];
    if window < 2 || window > n {
        return out;
    }
    for end in window..=n {
        let start = end - window;
        out[end - 1] = Some(pearson(&xs[start..end], &ys[start..end]));
    }
    out
}

/// One named return series and its pairwise correlations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub names: Vec<String>,
    /// `matrix[i][j]` = correlation of `names[i]` with `names[j]` (diagonal 1.0).
    pub matrix: Vec<Vec<f64>>,
}

/// Build a full pairwise correlation matrix over named return series.
pub fn correlation_matrix(series: &BTreeMap<String, Vec<f64>>) -> CorrelationMatrix {
    let names: Vec<String> = series.keys().cloned().collect();
    let cols: Vec<&Vec<f64>> = names.iter().map(|n| &series[n]).collect();
    let matrix = (0..names.len())
        .map(|i| {
            (0..names.len())
                .map(|j| if i == j { 1.0 } else { pearson(cols[i], cols[j]) })
                .collect()
        })
        .collect();
    CorrelationMatrix { names, matrix }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_positive_and_negative() {
        assert!((pearson(&[1.0, 2.0, 3.0, 4.0], &[2.0, 4.0, 6.0, 8.0]) - 1.0).abs() < 1e-12);
        assert!((pearson(&[1.0, 2.0, 3.0, 4.0], &[8.0, 6.0, 4.0, 2.0]) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn zero_variance_is_zero_not_nan() {
        let r = pearson(&[5.0, 5.0, 5.0], &[1.0, 2.0, 3.0]);
        assert_eq!(r, 0.0);
    }

    #[test]
    fn beta_of_market_against_itself_is_one() {
        let m = [0.01, -0.02, 0.015, -0.005, 0.02];
        assert!((beta(&m, &m) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn beta_scales_with_amplitude() {
        let m = [0.01, -0.02, 0.015, -0.005, 0.02];
        let a: Vec<f64> = m.iter().map(|x| x * 2.0).collect();
        // a = 2*m => beta = 2
        assert!((beta(&a, &m) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn rolling_warmup_then_values() {
        let xs = [1.0, 2.0, 3.0, 4.0, 5.0];
        let ys = [2.0, 4.0, 6.0, 8.0, 10.0];
        let r = rolling_pearson(&xs, &ys, 3);
        assert!(r[0].is_none() && r[1].is_none());
        assert!((r[2].unwrap() - 1.0).abs() < 1e-12);
        assert!((r[4].unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn matrix_diagonal_is_one() {
        let mut s = BTreeMap::new();
        s.insert("A".to_string(), vec![1.0, 2.0, 3.0, 4.0]);
        s.insert("B".to_string(), vec![4.0, 3.0, 2.0, 1.0]);
        let m = correlation_matrix(&s);
        assert_eq!(m.names, vec!["A".to_string(), "B".to_string()]);
        assert_eq!(m.matrix[0][0], 1.0);
        assert!((m.matrix[0][1] + 1.0).abs() < 1e-12); // A vs B anti-correlated
    }
}
