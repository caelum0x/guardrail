//! Moving Average Convergence Divergence (MACD).

use crate::ema::ema;

/// MACD over `(fast, slow, signal)` periods (classically 12, 26, 9).
///
/// Returns `(macd_line, signal_line, histogram)`, each aligned with `data`:
/// - `macd_line = EMA(fast) - EMA(slow)`
/// - `signal_line = EMA(signal)` of the MACD line
/// - `histogram = macd_line - signal_line`
///
/// # Panics
/// Panics if any period is `0`.
pub fn macd(
    data: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    assert!(fast > 0 && slow > 0 && signal > 0, "macd periods must be > 0");
    let n = data.len();
    let ef = ema(data, fast);
    let es = ema(data, slow);

    let mut macd_line = vec![f64::NAN; n];
    for i in 0..n {
        if !ef[i].is_nan() && !es[i].is_nan() {
            macd_line[i] = ef[i] - es[i];
        }
    }

    // Signal = EMA(signal) over the valid (non-NaN) portion of the MACD line.
    let mut signal_line = vec![f64::NAN; n];
    if let Some(start) = macd_line.iter().position(|v| !v.is_nan()) {
        let valid_ema = ema(&macd_line[start..], signal);
        for (offset, v) in valid_ema.into_iter().enumerate() {
            signal_line[start + offset] = v;
        }
    }

    let mut histogram = vec![f64::NAN; n];
    for i in 0..n {
        if !macd_line[i].is_nan() && !signal_line[i].is_nan() {
            histogram[i] = macd_line[i] - signal_line[i];
        }
    }

    (macd_line, signal_line, histogram)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macd_line_positive_in_uptrend() {
        let data: Vec<f64> = (1..=60).map(|x| x as f64).collect();
        let (line, signal, hist) = macd(&data, 12, 26, 9);
        let last = data.len() - 1;
        // Fast EMA above slow EMA in a steady uptrend -> positive MACD line.
        assert!(line[last] > 0.0);
        assert!(!signal[last].is_nan());
        // histogram defined once both lines exist
        assert!(!hist[last].is_nan());
    }

    #[test]
    fn macd_warmup_aligned() {
        let data: Vec<f64> = (1..=40).map(|x| x as f64).collect();
        let (line, _signal, _hist) = macd(&data, 12, 26, 9);
        // slow EMA seeds at index 25 -> macd_line valid from 25
        assert!(line[24].is_nan());
        assert!(!line[25].is_nan());
    }
}
