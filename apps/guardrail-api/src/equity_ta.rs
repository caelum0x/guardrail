//! Indicators over the live equity curve: `GET /equity/indicators`.
//!
//! Computes a technical indicator (via the real `ta-signals` crate) over the
//! agent's actual NAV series — the `PortfolioReconciled` events in the event
//! log — rather than a caller-supplied series. Read-only; no behavior change to
//! the trading loop. This ties the TA library to real agent data.
//!
//! `?indicator=sma|ema|rsi` (default `sma`), `?period=N` (default 14).

use axum::extract::{Query, State};
use axum::Json;
use event_store::AgentEvent;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::routes::AppState;

const RECENT_LIMIT: usize = 500;

#[derive(Debug, Deserialize)]
pub struct EquityTaQuery {
    indicator: Option<String>,
    period: Option<usize>,
}

fn to_json_series(values: &[f64]) -> Vec<Value> {
    values
        .iter()
        .map(|v| if v.is_nan() { Value::Null } else { json!(v) })
        .collect()
}

pub async fn equity_indicators(
    State(state): State<AppState>,
    Query(q): Query<EquityTaQuery>,
) -> Json<Value> {
    let events = match state.recent_events(RECENT_LIMIT) {
        Ok(events) => events,
        Err(error) => return Json(json!({ "error": error.to_string() })),
    };

    // NAV series in chronological order (recent() is newest-first).
    let series: Vec<f64> = events
        .iter()
        .rev()
        .filter(|e| matches!(e.event_type, AgentEvent::PortfolioReconciled))
        .filter_map(|e| {
            e.payload_json
                .get("nav_usd")
                .and_then(Value::as_str)
                .and_then(|s| s.parse::<f64>().ok())
        })
        .collect();

    let indicator = q.indicator.unwrap_or_else(|| "sma".into()).to_lowercase();
    let period = q.period.unwrap_or(14).max(1);

    if series.len() < 2 {
        return Json(json!({
            "error": "not enough NAV points in the event log — run the agent first",
            "nav_points": series.len(),
        }));
    }

    let result = match indicator.as_str() {
        "sma" => json!({ "values": to_json_series(&ta_signals::sma(&series, period)) }),
        "ema" => json!({ "values": to_json_series(&ta_signals::ema(&series, period)) }),
        "rsi" => json!({ "values": to_json_series(&ta_signals::rsi(&series, period)) }),
        other => {
            return Json(json!({
                "error": format!("unknown indicator '{other}'"),
                "supported": ["sma", "ema", "rsi"],
            }));
        }
    };

    Json(json!({
        "indicator": indicator,
        "period": period,
        "nav_points": series.len(),
        "result": result,
    }))
}
