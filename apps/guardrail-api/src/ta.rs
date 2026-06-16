//! Technical-analysis compute endpoint: `GET /ta`.
//!
//! Computes a technical indicator over a caller-supplied close-price series
//! using the real `ta-signals` crate (the same library the strategy could use).
//! Read-only and pure — no state, no external calls.
//!
//! Query params:
//! - `indicator` — one of `sma|ema|rsi|macd|bollinger` (close-series indicators)
//! - `series`    — comma-separated f64 close prices (e.g. `1,2,3,4,5`)
//! - `period`    — lookback (default 14; ignored by macd which uses 12/26/9)
//! - `mult`      — Bollinger std-dev multiplier (default 2.0)

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct TaParams {
    indicator: String,
    series: String,
    #[serde(default = "default_period")]
    period: usize,
    #[serde(default = "default_mult")]
    mult: f64,
}

fn default_period() -> usize {
    14
}
fn default_mult() -> f64 {
    2.0
}

/// Parse a comma-separated list of f64s, ignoring blanks.
fn parse_series(raw: &str) -> Vec<f64> {
    raw.split(',')
        .filter_map(|s| s.trim().parse::<f64>().ok())
        .collect()
}

/// Replace NaN with JSON null so the array round-trips cleanly.
fn to_json_series(values: &[f64]) -> Vec<Value> {
    values
        .iter()
        .map(|v| if v.is_nan() { Value::Null } else { json!(v) })
        .collect()
}

pub async fn ta(Query(p): Query<TaParams>) -> Json<Value> {
    let series = parse_series(&p.series);
    if series.is_empty() {
        return Json(json!({
            "error": "series must be a comma-separated list of numbers, e.g. series=1,2,3,4,5",
        }));
    }
    if p.period == 0 {
        return Json(json!({ "error": "period must be > 0" }));
    }

    let indicator = p.indicator.to_lowercase();
    let result = match indicator.as_str() {
        "sma" => json!({ "values": to_json_series(&ta_signals::sma(&series, p.period)) }),
        "ema" => json!({ "values": to_json_series(&ta_signals::ema(&series, p.period)) }),
        "rsi" => json!({ "values": to_json_series(&ta_signals::rsi(&series, p.period)) }),
        "macd" => {
            let (macd, signal, hist) = ta_signals::macd(&series, 12, 26, 9);
            json!({
                "macd": to_json_series(&macd),
                "signal": to_json_series(&signal),
                "histogram": to_json_series(&hist),
                "params": { "fast": 12, "slow": 26, "signal": 9 },
            })
        }
        "bollinger" => {
            let (upper, middle, lower) = ta_signals::bollinger(&series, p.period, p.mult);
            json!({
                "upper": to_json_series(&upper),
                "middle": to_json_series(&middle),
                "lower": to_json_series(&lower),
                "mult": p.mult,
            })
        }
        other => {
            return Json(json!({
                "error": format!("unknown or candle-only indicator '{other}'"),
                "supported": ["sma", "ema", "rsi", "macd", "bollinger"],
            }));
        }
    };

    Json(json!({
        "indicator": indicator,
        "period": p.period,
        "input_len": series.len(),
        "result": result,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_series_and_skips_blanks() {
        assert_eq!(parse_series("1, 2 ,3"), vec![1.0, 2.0, 3.0]);
        assert!(parse_series("").is_empty());
    }

    #[test]
    fn nan_maps_to_null() {
        let v = to_json_series(&[f64::NAN, 1.5]);
        assert_eq!(v[0], Value::Null);
        assert_eq!(v[1], json!(1.5));
    }
}
