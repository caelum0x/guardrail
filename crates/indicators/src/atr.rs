//! Average True Range (ATR) using Wilder's smoothing.

/// Average True Range (ATR), Wilder's smoothing method.
///
/// `high`, `low`, and `close` must all be the same length and represent
/// aligned candle data. The output vector has that same length.
///
/// The True Range for bar `i > 0` is:
/// `max(high - low, |high - prev_close|, |low - prev_close|)`.
/// For bar `0` (no previous close) it is `high - low`.
///
/// ATR is seeded with the SMA of the first `period` true ranges, then smoothed
/// with Wilder's recurrence. Warm-up positions repeat the seed so the series is
/// aligned and free of NaN/None.
///
/// Returns an empty vector when:
/// - `period == 0`,
/// - the three slices differ in length,
/// - or there is insufficient data (`len < period + 1`, since the first true
///   range that uses a previous close is at index 1).
#[must_use]
pub fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let len = high.len();
    if period == 0 || len != low.len() || len != close.len() || len < period + 1 {
        return Vec::new();
    }

    // True range series, same length as inputs; index 0 has no previous close.
    let mut tr = vec![0.0_f64; len];
    tr[0] = high[0] - low[0];
    for i in 1..len {
        let prev_close = close[i - 1];
        let hl = high[i] - low[i];
        let hc = (high[i] - prev_close).abs();
        let lc = (low[i] - prev_close).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    let mut out = vec![0.0_f64; len];

    // Seed ATR at index `period` with the SMA of true ranges over 1..=period.
    let seed: f64 = tr[1..=period].iter().sum::<f64>() / period as f64;

    for slot in out.iter_mut().take(period + 1) {
        *slot = seed;
    }

    let pf = period as f64;
    let mut prev = seed;
    for i in (period + 1)..len {
        let next = (prev * (pf - 1.0) + tr[i]) / pf;
        out[i] = next;
        prev = next;
    }

    out
}
