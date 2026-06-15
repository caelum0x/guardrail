//! Scenario stress endpoint.
//!
//! Applies product-owned category shocks to the latest run report positions.
//! It is a read-only desk for pre-trade and demo risk review.

use axum::Json;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const DEFAULT_REPORT: &str = "data/run_report.json";
const SCENARIOS: &str = "configs/scenarios/market_stress.json";

pub async fn scenarios() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into());
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&report_path)?)?;
    let universe: Value = serde_json::from_str(&std::fs::read_to_string(UNIVERSE)?)?;
    let scenarios: Value = serde_json::from_str(&std::fs::read_to_string(SCENARIOS)?)?;
    let category_by_symbol = category_map(&universe);
    let nav_usd = report
        .get("nav_usd")
        .and_then(Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO);
    let positions = report
        .get("positions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut results = Vec::new();
    let mut worst_id = String::new();
    let mut worst_pnl = Decimal::MAX;

    for scenario in scenarios.as_array().cloned().unwrap_or_default() {
        let id = scenario
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let shocks = shock_map(
            scenario
                .get("category_shocks_pct")
                .unwrap_or(&Value::Object(Default::default())),
        );
        let mut rows = Vec::new();
        let mut total_pnl = Decimal::ZERO;
        let mut largest_loss = json!({ "symbol": null, "pnl_usd": "0" });
        let mut largest_loss_value = Decimal::ZERO;

        for position in &positions {
            let symbol = position
                .get("symbol")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_string();
            let value_usd = decimal_field(position, "value_usd");
            let weight_pct = decimal_field(position, "weight_pct");
            let category = category_by_symbol
                .get(&symbol)
                .cloned()
                .unwrap_or_else(|| "uncategorized".to_string());
            let shock_pct = shocks
                .get(&category)
                .copied()
                .or_else(|| shocks.get("uncategorized").copied())
                .unwrap_or(Decimal::ZERO);
            let pnl_usd = (value_usd * shock_pct / Decimal::from(100)).round_dp(2);
            let stressed_value_usd = (value_usd + pnl_usd).max(Decimal::ZERO).round_dp(2);
            total_pnl += pnl_usd;
            if pnl_usd < largest_loss_value {
                largest_loss_value = pnl_usd;
                largest_loss = json!({
                    "symbol": symbol,
                    "category": category,
                    "pnl_usd": pnl_usd.to_string(),
                    "shock_pct": shock_pct.to_string()
                });
            }
            rows.push(json!({
                "symbol": symbol,
                "category": category,
                "weight_pct": weight_pct.round_dp(2).to_string(),
                "value_usd": value_usd.round_dp(2).to_string(),
                "shock_pct": shock_pct.to_string(),
                "pnl_usd": pnl_usd.to_string(),
                "stressed_value_usd": stressed_value_usd.to_string()
            }));
        }

        let portfolio_return_pct = if nav_usd > Decimal::ZERO {
            (total_pnl / nav_usd * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        if total_pnl < worst_pnl {
            worst_pnl = total_pnl;
            worst_id = id.clone();
        }
        let status = if portfolio_return_pct <= Decimal::from(-15) {
            "critical"
        } else if portfolio_return_pct <= Decimal::from(-7) {
            "watch"
        } else {
            "normal"
        };

        results.push(json!({
            "id": id,
            "label": scenario.get("label").cloned().unwrap_or(json!("Unknown")),
            "description": scenario.get("description").cloned().unwrap_or(json!("")),
            "status": status,
            "portfolio_pnl_usd": total_pnl.round_dp(2).to_string(),
            "portfolio_return_pct": portfolio_return_pct.to_string(),
            "largest_loss": largest_loss,
            "positions": rows
        }));
    }

    Ok(json!({
        "report_path": report_path,
        "universe_path": UNIVERSE,
        "scenarios_path": SCENARIOS,
        "nav_usd": nav_usd.round_dp(2).to_string(),
        "worst_scenario_id": worst_id,
        "worst_pnl_usd": worst_pnl.round_dp(2).to_string(),
        "scenarios": results
    }))
}

fn category_map(universe: &Value) -> BTreeMap<String, String> {
    let mut categories = BTreeMap::new();
    if let Some(assets) = universe.as_array() {
        for asset in assets {
            let Some(symbol) = asset.get("symbol").and_then(Value::as_str) else {
                continue;
            };
            let Some(category) = asset.get("category").and_then(Value::as_str) else {
                continue;
            };
            categories.insert(symbol.to_string(), category.to_string());
        }
    }
    categories
}

fn shock_map(value: &Value) -> BTreeMap<String, Decimal> {
    let mut shocks = BTreeMap::new();
    if let Some(object) = value.as_object() {
        for (category, shock) in object {
            let parsed = shock
                .as_f64()
                .and_then(Decimal::from_f64)
                .or_else(|| shock.as_str().and_then(decimal_from_str))
                .unwrap_or(Decimal::ZERO);
            shocks.insert(category.clone(), parsed);
        }
    }
    shocks
}

fn decimal_field(value: &Value, field: &str) -> Decimal {
    value
        .get(field)
        .and_then(Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO)
}

fn decimal_from_str(value: &str) -> Option<Decimal> {
    Decimal::from_str(value).ok()
}
