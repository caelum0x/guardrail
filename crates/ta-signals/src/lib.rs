//! Pure-Rust technical-analysis signals.
//!
//! Every indicator takes a price (`&[f64]`) or candle (`&[Candle]`) series and
//! returns a `Vec` aligned with the input, with `NaN` during the warmup window
//! so callers can zip results back to their bars without index bookkeeping.
//!
//! ```
//! use ta_signals::{sma, rsi};
//! let closes = [1.0, 2.0, 3.0, 4.0, 5.0];
//! let avg = sma(&closes, 3);
//! assert!((avg[2] - 2.0).abs() < 1e-12);
//! assert!(avg[0].is_nan()); // warmup
//! let _ = rsi(&closes, 2);
//! ```

mod adx;
mod atr;
mod bollinger;
mod candle;
mod ema;
mod macd;
mod obv;
mod rsi;
mod sma;
mod stochastic;
mod vwap;

pub use adx::adx;
pub use atr::atr;
pub use bollinger::bollinger;
pub use candle::{closes, highs, lows, volumes, Candle};
pub use ema::ema;
pub use macd::macd;
pub use obv::obv;
pub use rsi::rsi;
pub use sma::sma;
pub use stochastic::stochastic;
pub use vwap::vwap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indicators_align_to_input_length() {
        let closes: Vec<f64> = (1..=60).map(|x| x as f64).collect();
        assert_eq!(sma(&closes, 14).len(), closes.len());
        assert_eq!(ema(&closes, 14).len(), closes.len());
        assert_eq!(rsi(&closes, 14).len(), closes.len());
        let (m, s, h) = macd(&closes, 12, 26, 9);
        assert_eq!(m.len(), closes.len());
        assert_eq!(s.len(), closes.len());
        assert_eq!(h.len(), closes.len());
        let (u, mid, l) = bollinger(&closes, 20, 2.0);
        assert_eq!(u.len(), closes.len());
        assert_eq!(mid.len(), closes.len());
        assert_eq!(l.len(), closes.len());

        let candles: Vec<Candle> = closes
            .iter()
            .map(|&c| Candle::new(c, c + 1.0, c - 1.0, c, 100.0))
            .collect();
        assert_eq!(atr(&candles, 14).len(), candles.len());
        assert_eq!(adx(&candles, 14).len(), candles.len());
        assert_eq!(obv(&candles).len(), candles.len());
        assert_eq!(vwap(&candles).len(), candles.len());
        let (k, d) = stochastic(&candles, 14, 3);
        assert_eq!(k.len(), candles.len());
        assert_eq!(d.len(), candles.len());
    }
}
