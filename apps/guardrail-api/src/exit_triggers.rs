//! Exit trigger endpoint.
//!
//! Evaluates current report positions against configured stop-loss,
//! take-profit, and market-move triggers. Advisory and read-only.

use axum::Json;
use common::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::{json, Value};

const POLICY: &str = "configs/exits/exit_policy.json";

pub async fn exit_triggers() -> Json<Value> {
    match build().await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build() -> anyhow::Result<Value> {
    let policy: Value = serde_json::from_str(&std::fs::read_to_string(POLICY)?)?;
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| {
        policy
            .get("report_path")
            .and_then(Value::as_str)
            .unwrap_or("data/run_report.json")
            .to_string()
    });
    let universe_path = policy
        .get("universe_path")
        .and_then(Value::as_str)
        .unwrap_or("configs/eligible_assets.bsc.json");
    let stop_loss = decimal_config(&policy, "stop_loss_pct", Decimal::from(12));
    let take_profit = decimal_config(&policy, "take_profit_pct", Decimal::from(25));
    let warning_loss = decimal_config(&policy, "warning_loss_pct", Decimal::from(8));
    let warning_gain = decimal_config(&policy, "warning_gain_pct", Decimal::from(18));
    let ret_exit = decimal_config(&policy, "ret_24h_exit_pct", Decimal::from(-12));

    let report: Value = serde_json::from_str(&std::fs::read_to_string(&report_path)?)?;
    let universe = market_data::Universe::load(universe_path)?;
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;

    let mut rows = Vec::new();
    let mut exits = 0usize;
    let mut watches = 0usize;
    for position in report
        .get("positions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let symbol = position.get("symbol").and_then(Value::as_str).unwrap_or("");
        let value_usd = decimal_field(&position, "value_usd");
        let weight_pct = decimal_field(&position, "weight_pct");
        let state = snapshot.get(symbol);
        let ret_24h = state
            .and_then(|asset| asset.ret_24h)
            .unwrap_or(Decimal::ZERO);
        let synthetic_pnl_pct = ret_24h;
        let reason = if synthetic_pnl_pct <= -stop_loss || ret_24h <= ret_exit {
            exits += 1;
            "exit"
        } else if synthetic_pnl_pct >= take_profit {
            exits += 1;
            "take_profit"
        } else if synthetic_pnl_pct <= -warning_loss || synthetic_pnl_pct >= warning_gain {
            watches += 1;
            "watch"
        } else {
            "hold"
        };
        rows.push(json!({
            "symbol": symbol,
            "status": reason,
            "value_usd": value_usd.round_dp(2).to_string(),
            "weight_pct": weight_pct.round_dp(2).to_string(),
            "ret_24h": ret_24h.round_dp(2).to_string(),
            "synthetic_pnl_pct": synthetic_pnl_pct.round_dp(2).to_string(),
            "price_usd": state.map(|asset| asset.price_usd.round_dp(6).to_string()),
            "safety_score": state.map(|asset| asset.safety_score)
        }));
    }
    rows.sort_by(|a, b| {
        decimal_field(b, "synthetic_pnl_pct").cmp(&decimal_field(a, "synthetic_pnl_pct"))
    });
    Ok(json!({
        "policy_path": POLICY,
        "report_path": report_path,
        "universe_path": universe_path,
        "thresholds": {
            "stop_loss_pct": stop_loss.to_string(),
            "take_profit_pct": take_profit.to_string(),
            "warning_loss_pct": warning_loss.to_string(),
            "warning_gain_pct": warning_gain.to_string(),
            "ret_24h_exit_pct": ret_exit.to_string()
        },
        "summary": {
            "positions": rows.len(),
            "exit": exits,
            "watch": watches,
            "hold": rows.len().saturating_sub(exits + watches)
        },
        "positions": rows
    }))
}

fn decimal_config(config: &Value, key: &str, default: Decimal) -> Decimal {
    config
        .get(key)
        .and_then(Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| config.get(key).and_then(Value::as_i64).map(Decimal::from))
        .or_else(|| config.get(key).and_then(Value::as_u64).map(Decimal::from))
        .or_else(|| {
            config
                .get(key)
                .and_then(Value::as_str)
                .and_then(decimal_from_str)
        })
        .unwrap_or(default)
}

fn decimal_field(value: &Value, key: &str) -> Decimal {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO)
}

fn decimal_from_str(value: &str) -> Option<Decimal> {
    Decimal::from_str(value).ok()
}
