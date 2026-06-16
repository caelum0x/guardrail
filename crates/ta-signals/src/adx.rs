//! Average Directional Index (ADX), Wilder's directional system.

use crate::candle::Candle;

/// Wilder's ADX over `period` (classically 14). Aligned with `candles`; the
/// first ADX value lands at index `2*period - 1` (one `period` to smooth the
/// directional movement, another to average DX into ADX). Returns values in
/// `[0, 100]`; entries before the warmup are `NaN`.
///
/// # Panics
/// Panics if `period == 0`.
pub fn adx(candles: &[Candle], period: usize) -> Vec<f64> {
    assert!(period > 0, "adx period must be > 0");
    let n = candles.len();
    let mut out = vec![f64::NAN; n];
    if n <= 2 * period {
        return out;
    }
    let p = period as f64;

    // Directional movement and true range per bar.
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];
    let mut tr = vec![0.0; n];
    for i in 1..n {
        let up = candles[i].high - candles[i - 1].high;
        let down = candles[i - 1].low - candles[i].low;
        plus_dm[i] = if up > down && up > 0.0 { up } else { 0.0 };
        minus_dm[i] = if down > up && down > 0.0 { down } else { 0.0 };
        let (h, l, pc) = (candles[i].high, candles[i].low, candles[i - 1].close);
        tr[i] = (h - l).max((h - pc).abs()).max((l - pc).abs());
    }

    // Wilder-smoothed running sums (seed at index `period`).
    let (mut sp, mut sm, mut st) = (
        plus_dm[1..=period].iter().sum::<f64>(),
        minus_dm[1..=period].iter().sum::<f64>(),
        tr[1..=period].iter().sum::<f64>(),
    );
    let mut dx = vec![f64::NAN; n];
    let dx_at = |sp: f64, sm: f64, st: f64| -> f64 {
        if st == 0.0 {
            return 0.0;
        }
        let pdi = 100.0 * sp / st;
        let mdi = 100.0 * sm / st;
        let sum = pdi + mdi;
        if sum == 0.0 {
            0.0
        } else {
            100.0 * (pdi - mdi).abs() / sum
        }
    };
    dx[period] = dx_at(sp, sm, st);
    for (i, dx_i) in dx.iter_mut().enumerate().take(n).skip(period + 1) {
        sp = sp - sp / p + plus_dm[i];
        sm = sm - sm / p + minus_dm[i];
        st = st - st / p + tr[i];
        *dx_i = dx_at(sp, sm, st);
    }

    // ADX = Wilder average of DX, seeded over the first `period` DX values.
    let first_adx = 2 * period - 1;
    let mut adx = dx[period..=first_adx].iter().sum::<f64>() / p;
    out[first_adx] = adx;
    for (i, out_i) in out.iter_mut().enumerate().take(n).skip(first_adx + 1) {
        adx = (adx * (p - 1.0) + dx[i]) / p;
        *out_i = adx;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_uptrend_has_high_adx() {
        // Steadily rising highs/lows -> strong trend -> ADX climbs well above 25.
        let candles: Vec<Candle> = (0..50)
            .map(|i| {
                let base = i as f64;
                Candle::new(base, base + 1.0, base, base + 0.5, 1.0)
            })
            .collect();
        let r = adx(&candles, 14);
        let last = *r.last().unwrap();
        assert!(last.is_finite());
        assert!(last > 25.0, "trend ADX should be strong, got {last}");
        assert!((0.0..=100.0).contains(&last));
    }

    #[test]
    fn warmup_is_nan_until_2period_minus_1() {
        let candles: Vec<Candle> = (0..40)
            .map(|i| Candle::new(i as f64, i as f64 + 1.0, i as f64, i as f64 + 0.5, 1.0))
            .collect();
        let r = adx(&candles, 14);
        for v in r.iter().take(2 * 14 - 1) {
            assert!(v.is_nan());
        }
        assert!(r[27].is_finite());
    }
}
