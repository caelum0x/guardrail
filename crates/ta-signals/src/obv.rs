//! On-Balance Volume (OBV).

use crate::candle::Candle;

/// On-Balance Volume: a running total that adds the bar's volume on an up-close
/// and subtracts it on a down-close. Aligned with `candles`, starting at `0.0`
/// for the first bar (no warmup NaN — OBV is cumulative from the first bar).
pub fn obv(candles: &[Candle]) -> Vec<f64> {
    let n = candles.len();
    let mut out = vec![0.0; n];
    for i in 1..n {
        let (prev, cur) = (candles[i - 1].close, candles[i].close);
        out[i] = out[i - 1]
            + match cur.partial_cmp(&prev) {
                Some(std::cmp::Ordering::Greater) => candles[i].volume,
                Some(std::cmp::Ordering::Less) => -candles[i].volume,
                _ => 0.0,
            };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(close: f64, vol: f64) -> Candle {
        Candle::new(close, close, close, close, vol)
    }

    #[test]
    fn obv_accumulates_on_direction() {
        let candles = vec![
            bar(10.0, 100.0),
            bar(11.0, 50.0),  // up   -> +50
            bar(10.5, 30.0),  // down -> -30
            bar(10.5, 20.0),  // flat -> 0
            bar(12.0, 40.0),  // up   -> +40
        ];
        let r = obv(&candles);
        assert_eq!(r, vec![0.0, 50.0, 20.0, 20.0, 60.0]);
    }
}
