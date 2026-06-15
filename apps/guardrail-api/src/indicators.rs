//! Technical-indicators endpoint.
//!
//! Builds a deterministic synthetic close series for a symbol and computes a
//! suite of classic technical indicators (EMA, SMA, RSI, MACD, Bollinger
//! Bands) over it. Read-only and side-effect free — it never touches the live
//! book, the event log, or any external data source.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

/// Sentiment value driving the synthetic price drift. Neutral so the series is
/// dominated by volatility rather than a directional trend.
const FEAR_GREED: u32 = 60;

#[derive(Debug, Deserialize)]
pub struct IndicatorParams {
    /// Asset symbol to chart (default "WBNB").
    pub symbol: Option<String>,
    /// Number of close samples to generate (default 60, clamped 10..=500).
    pub steps: Option<u32>,
}

pub async fn indicators(Query(params): Query<IndicatorParams>) -> Json<Value> {
    Json(build(&params))
}

/// Evolve a deterministic close series and compute indicators over it.
fn build(params: &IndicatorParams) -> Value {
    let symbol = params
        .symbol
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("WBNB")
        .to_string();
    let steps = params.steps.unwrap_or(60).clamp(10, 500);

    let closes = build_closes(&symbol, steps);

    let ema = indicators::ema(&closes, 12);
    let sma = indicators::sma(&closes, 12);
    let rsi = indicators::rsi(&closes, 14);
    let macd = indicators::macd(&closes, 12, 26, 9);
    let bollinger = indicators::bollinger(&closes, 20, 2.0);

    json!({
        "symbol": symbol,
        "steps": steps,
        "closes": closes,
        "ema": ema,
        "sma": sma,
        "rsi": rsi,
        "macd": {
            "macd": macd.macd,
            "signal": macd.signal,
            "histogram": macd.histogram,
        },
        "bollinger": {
            "mid": bollinger.mid,
            "upper": bollinger.upper,
            "lower": bollinger.lower,
        },
    })
}

/// Deterministically evolve a price path of `steps` closes for `symbol`.
fn build_closes(symbol: &str, steps: u32) -> Vec<f64> {
    let mut price = backtester::synthetic::initial_price(symbol);
    let mut closes = Vec::with_capacity(steps as usize);
    for step in 0..steps {
        let ret_pct = backtester::synthetic::step_return_24h_pct(symbol, step, FEAR_GREED);
        price = common::decimal::apply_pct(price, rust_decimal::Decimal::from(100) + ret_pct);
        closes.push(common::decimal::to_f64(price));
    }
    closes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_closes_is_deterministic_and_sized() {
        let a = build_closes("WBNB", 30);
        let b = build_closes("WBNB", 30);
        assert_eq!(a.len(), 30);
        assert_eq!(a, b);
        assert!(a.iter().all(|p| p.is_finite() && *p > 0.0));
    }

    #[test]
    fn build_clamps_steps_and_defaults_symbol() {
        let params = IndicatorParams {
            symbol: None,
            steps: Some(5),
        };
        let value = build(&params);
        assert_eq!(value["symbol"], "WBNB");
        assert_eq!(value["steps"], 10);
    }
}
