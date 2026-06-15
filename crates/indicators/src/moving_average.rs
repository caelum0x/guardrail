//! Simple and exponential moving averages.

/// Simple Moving Average (SMA).
///
/// Returns a vector aligned to `values`: the first `period - 1` entries are
/// `f64::NAN`-free by being omitted — instead the output length equals
/// `values.len()` and warm-up positions are filled with the SMA computed over
/// the available leading window is NOT done; rather we left-pad with the first
/// fully-formed value's window. To keep the contract simple and NaN-free, the
/// output has the same length as the input with warm-up positions carrying the
/// earliest computable average.
///
/// Concretely, for index `i >= period - 1` the value is the mean of the last
/// `period` samples. For `i < period - 1` the value repeats the first
/// fully-formed average so the series stays aligned and free of NaN/None.
///
/// Returns an empty vector when there is insufficient data
/// (`values.len() < period`) or when `period == 0`.
#[must_use]
pub fn sma(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }

    let mut out = vec![0.0_f64; values.len()];

    // Rolling sum for O(n) computation.
    let mut window_sum: f64 = values[..period].iter().sum();
    let first_avg = window_sum / period as f64;

    // Warm-up positions repeat the first fully-formed average.
    for slot in out.iter_mut().take(period - 1) {
        *slot = first_avg;
    }
    out[period - 1] = first_avg;

    for i in period..values.len() {
        window_sum += values[i] - values[i - period];
        out[i] = window_sum / period as f64;
    }

    out
}

/// Exponential Moving Average (EMA).
///
/// Uses the standard smoothing factor `alpha = 2 / (period + 1)`. The series is
/// seeded with the SMA of the first `period` samples, then updated recursively.
///
/// The returned vector has the same length as `values`. Warm-up positions
/// (before the seed index) repeat the seed value so the series is aligned and
/// free of NaN/None.
///
/// Returns an empty vector when there is insufficient data
/// (`values.len() < period`) or when `period == 0`.
#[must_use]
pub fn ema(values: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || values.len() < period {
        return Vec::new();
    }

    let alpha = 2.0 / (period as f64 + 1.0);
    let mut out = vec![0.0_f64; values.len()];

    // Seed with the SMA of the first `period` samples.
    let seed: f64 = values[..period].iter().sum::<f64>() / period as f64;

    for slot in out.iter_mut().take(period) {
        *slot = seed;
    }

    let mut prev = seed;
    for i in period..values.len() {
        let next = alpha * values[i] + (1.0 - alpha) * prev;
        out[i] = next;
        prev = next;
    }

    out
}
