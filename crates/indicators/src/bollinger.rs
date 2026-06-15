//! Bollinger Bands.

use serde::Serialize;

use crate::moving_average::sma;

/// Bollinger Bands output: the middle (SMA) band plus the upper and lower
/// bands offset by `k` standard deviations.
///
/// All three vectors share the same length as the input series (or are all
/// empty when there is insufficient data).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Bollinger {
    /// Middle band: the simple moving average.
    pub mid: Vec<f64>,
    /// Upper band: `mid + k * stddev`.
    pub upper: Vec<f64>,
    /// Lower band: `mid - k * stddev`.
    pub lower: Vec<f64>,
}

/// Compute Bollinger Bands over `values`.
///
/// `period` is the moving-average / standard-deviation window; `k` is the
/// number of standard deviations for the band offset (commonly `2.0`).
///
/// The standard deviation uses the population formula (dividing by `period`).
/// Warm-up positions (indices before `period - 1`) repeat the earliest
/// fully-formed band values so the series stays aligned and free of NaN/None.
///
/// Returns empty vectors when `period == 0` or when there is insufficient data
/// (`values.len() < period`).
#[must_use]
pub fn bollinger(values: &[f64], period: usize, k: f64) -> Bollinger {
    let empty = Bollinger {
        mid: Vec::new(),
        upper: Vec::new(),
        lower: Vec::new(),
    };

    if period == 0 || values.len() < period {
        return empty;
    }

    let mid = sma(values, period);
    if mid.is_empty() {
        return empty;
    }

    let mut upper = vec![0.0_f64; values.len()];
    let mut lower = vec![0.0_f64; values.len()];

    // Compute the first fully-formed band (at index period - 1) and reuse it
    // for the warm-up positions.
    let pf = period as f64;
    let std_at = |end: usize, mean: f64| -> f64 {
        // end is exclusive; window is values[end - period .. end].
        let start = end - period;
        let var: f64 = values[start..end]
            .iter()
            .map(|v| {
                let d = v - mean;
                d * d
            })
            .sum::<f64>()
            / pf;
        var.sqrt()
    };

    let first_std = std_at(period, mid[period - 1]);
    let first_upper = mid[period - 1] + k * first_std;
    let first_lower = mid[period - 1] - k * first_std;

    for i in 0..period {
        upper[i] = first_upper;
        lower[i] = first_lower;
    }

    for i in period..values.len() {
        let std = std_at(i + 1, mid[i]);
        upper[i] = mid[i] + k * std;
        lower[i] = mid[i] - k * std;
    }

    Bollinger { mid, upper, lower }
}
