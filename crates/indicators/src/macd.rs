//! Moving Average Convergence Divergence (MACD).

use serde::Serialize;

use crate::moving_average::ema;

/// MACD output: the MACD line, the signal line, and the histogram.
///
/// All three vectors share the same length as the input series (or are all
/// empty when there is insufficient data).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Macd {
    /// MACD line: `ema(fast) - ema(slow)`.
    pub macd: Vec<f64>,
    /// Signal line: `ema(signal)` of the MACD line.
    pub signal: Vec<f64>,
    /// Histogram: `macd - signal`.
    pub histogram: Vec<f64>,
}

/// Compute MACD over `values`.
///
/// `fast` and `slow` are the EMA periods for the MACD line (`fast` should be
/// smaller than `slow`); `signal` is the EMA period applied to the MACD line.
///
/// Returns a `Macd` whose three vectors are all aligned to `values`. Returns
/// empty vectors when any period is `0`, when `fast >= slow`, or when there is
/// insufficient data for the required EMAs.
#[must_use]
pub fn macd(values: &[f64], fast: usize, slow: usize, signal: usize) -> Macd {
    let empty = Macd {
        macd: Vec::new(),
        signal: Vec::new(),
        histogram: Vec::new(),
    };

    if fast == 0 || slow == 0 || signal == 0 || fast >= slow {
        return empty;
    }

    let fast_ema = ema(values, fast);
    let slow_ema = ema(values, slow);

    // ema returns empty on insufficient data; both must be populated and the
    // slow EMA governs the (longer) warm-up, so both are length == values.len().
    if fast_ema.is_empty() || slow_ema.is_empty() {
        return empty;
    }

    let macd_line: Vec<f64> = fast_ema
        .iter()
        .zip(slow_ema.iter())
        .map(|(f, s)| f - s)
        .collect();

    let signal_line = ema(&macd_line, signal);
    if signal_line.is_empty() {
        return empty;
    }

    let histogram: Vec<f64> = macd_line
        .iter()
        .zip(signal_line.iter())
        .map(|(m, s)| m - s)
        .collect();

    Macd {
        macd: macd_line,
        signal: signal_line,
        histogram,
    }
}
