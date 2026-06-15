//! Track-1 daily trade heartbeat planner.
//!
//! Read-only endpoint that checks whether the minimum daily-trade requirement
//! has evidence in the recent event log and proposes a tiny, risk-capped TWAK
//! heartbeat order when attention is needed.

use axum::{extract::State, Json};
use event_store::AgentEvent;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};

const CONFIG: &str = "configs/heartbeat/daily_trade.json";
const RECENT_LIMIT: usize = 500;

pub async fn heartbeat(State(state): State<crate::routes::AppState>) -> Json<Value> {
    match build(&state) {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build(state: &crate::routes::AppState) -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(CONFIG)?)?;
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| {
        config
            .get("report_path")
            .and_then(Value::as_str)
            .unwrap_or("data/run_report.json")
            .to_string()
    });
    let report: Value = std::fs::read_to_string(&report_path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let events = state.recent_events(RECENT_LIMIT).unwrap_or_default();
    let txs = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .collect::<Vec<_>>();
    let daily_marker = events
        .iter()
        .find(|event| matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied));
    let last_trade = txs.first();
    let nav = decimal_value(report.get("nav_usd")).unwrap_or(Decimal::from(10_000));
    let max_pct = decimal_value(config.get("max_heartbeat_trade_pct")).unwrap_or(Decimal::from(2));
    let min_notional = decimal_value(config.get("min_notional_usd")).unwrap_or(Decimal::from(25));
    let target_notional =
        decimal_value(config.get("target_notional_usd")).unwrap_or(Decimal::from(100));
    let max_notional = decimal_value(config.get("max_notional_usd")).unwrap_or(Decimal::from(200));
    let cap_by_nav = nav * max_pct / Decimal::from(100);
    let planned = target_notional
        .min(max_notional)
        .min(cap_by_nav)
        .max(min_notional);
    let min_trades = config
        .get("min_trades_per_day")
        .and_then(Value::as_u64)
        .unwrap_or(1) as usize;
    let satisfied = daily_marker.is_some() || txs.len() >= min_trades;
    let from = config
        .get("preferred_pair")
        .and_then(|pair| pair.get("from"))
        .and_then(Value::as_str)
        .unwrap_or("USDT");
    let to = config
        .get("preferred_pair")
        .and_then(|pair| pair.get("to"))
        .and_then(Value::as_str)
        .unwrap_or("WBNB");

    Ok(json!({
        "status": if satisfied { "satisfied" } else { "due" },
        "config_path": CONFIG,
        "report_path": report_path,
        "name": config.get("name").cloned().unwrap_or(json!("Daily Trade Heartbeat")),
        "requirement": {
            "min_trades_per_day": min_trades,
            "cooldown_hours": config.get("cooldown_hours").cloned().unwrap_or(json!(24)),
            "max_heartbeat_trade_pct": max_pct.to_string()
        },
        "evidence": {
            "recent_confirmed_txs": txs.len(),
            "daily_marker_present": daily_marker.is_some(),
            "last_trade_timestamp": last_trade.map(|event| event.timestamp.clone()),
            "last_trade_tx": last_trade.and_then(|event| event.payload_json.get("tx_hash")).and_then(Value::as_str),
            "last_marker_timestamp": daily_marker.map(|event| event.timestamp.clone())
        },
        "plan": {
            "needed": !satisfied,
            "from_symbol": from,
            "to_symbol": to,
            "notional_usd": planned.round_dp(2).to_string(),
            "nav_usd": nav.round_dp(2).to_string(),
            "execution_path": config.get("execution_path").cloned().unwrap_or(json!("risk_gate -> TWAK quote -> final risk -> TWAK swap")),
            "operator_command": format!("cargo run -p guardrail-cli -- quote --from {from} --to {to} --amount {}", planned.round_dp(2))
        }
    }))
}

fn decimal_value(value: Option<&Value>) -> Option<Decimal> {
    value
        .and_then(Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| value.and_then(Value::as_i64).map(Decimal::from))
        .or_else(|| value.and_then(Value::as_u64).map(Decimal::from))
        .or_else(|| {
            value
                .and_then(Value::as_str)
                .and_then(|raw| raw.parse::<Decimal>().ok())
        })
}
