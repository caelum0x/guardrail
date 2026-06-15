//! Relative Strength Index (RSI) using Wilder's smoothing.

/// Relative Strength Index (RSI), Wilder's smoothing method.
///
/// The output vector has the same length as `values`. The first computable RSI
/// appears at index `period`; warm-up positions (indices `0..period`) repeat
/// the first computed RSI value so the series is aligned and free of NaN/None.
///
/// Values are bounded in `[0.0, 100.0]`. A monotonically rising series produces
/// RSI values approaching `100.0`; a monotonically falling series approaches
/// `0.0`.
///
/// Returns an empty vector when there is insufficient data
/// (`values.len() <= period`) or when `period == 0`.
#[must_use]
pub fn rsi(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() <= period {
        return Vec::new();
    }

    let mut out = vec![0.0_f64; values.len()];

    // Average gain/loss over the first `period` changes (indices 1..=period).
    let mut gain_sum = 0.0_f64;
    let mut loss_sum = 0.0_f64;
    for i in 1..=period {
        let change = values[i] - values[i - 1];
        if change >= 0.0 {
            gain_sum += change;
        } else {
            loss_sum -= change; // accumulate magnitude
        }
    }

    let mut avg_gain = gain_sum / period as f64;
    let mut avg_loss = loss_sum / period as f64;

    let first_rsi = rsi_from_averages(avg_gain, avg_loss);

    // Warm-up positions repeat the first fully-formed RSI.
    for slot in out.iter_mut().take(period + 1) {
        *slot = first_rsi;
    }

    let pf = period as f64;
    for i in (period + 1)..values.len() {
        let change = values[i] - values[i - 1];
        let (gain, loss) = if change >= 0.0 {
            (change, 0.0)
        } else {
            (0.0, -change)
        };
        avg_gain = (avg_gain * (pf - 1.0) + gain) / pf;
        avg_loss = (avg_loss * (pf - 1.0) + loss) / pf;
        out[i] = rsi_from_averages(avg_gain, avg_loss);
    }

    out
}

/// Compute RSI from average gain/loss, handling the zero-loss edge case.
fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        // No losses: maximally overbought (or flat). 100 by convention.
        if avg_gain == 0.0 {
            // Completely flat series: neutral midpoint.
            50.0
        } else {
            100.0
        }
    } else {
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}
