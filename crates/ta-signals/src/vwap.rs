//! Volume-Weighted Average Price (VWAP), cumulative.

use crate::candle::Candle;

/// Cumulative VWAP: `sum(typical_price * volume) / sum(volume)` accumulated from
/// the first bar. Aligned with `candles`. A bar is `NaN` only while cumulative
/// volume is still zero.
pub fn vwap(candles: &[Candle]) -> Vec<f64> {
    let n = candles.len();
    let mut out = vec![f64::NAN; n];
    let (mut cum_pv, mut cum_v) = (0.0, 0.0);
    for i in 0..n {
        cum_pv += candles[i].typical_price() * candles[i].volume;
        cum_v += candles[i].volume;
        if cum_v != 0.0 {
            out[i] = cum_pv / cum_v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vwap_of_constant_price_is_that_price() {
        let candles: Vec<Candle> = (0..5).map(|_| Candle::new(10.0, 10.0, 10.0, 10.0, 7.0)).collect();
        let r = vwap(&candles);
        for v in r {
            assert!((v - 10.0).abs() < 1e-12);
        }
    }

    #[test]
    fn vwap_weights_by_volume() {
        // tp = close here. Bar1: 10 @ vol1, Bar2: 20 @ vol3.
        let candles = vec![
            Candle::new(10.0, 10.0, 10.0, 10.0, 1.0),
            Candle::new(20.0, 20.0, 20.0, 20.0, 3.0),
        ];
        let r = vwap(&candles);
        // (10*1 + 20*3) / (1+3) = 70/4 = 17.5
        assert!((r[1] - 17.5).abs() < 1e-12);
    }
}
