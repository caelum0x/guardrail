//! Current exposure endpoint.
//!
//! Joins the latest run report positions with the BSC eligible universe so the
//! operator can see category concentration before the next rebalance.

use axum::Json;
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const DEFAULT_REPORT: &str = "data/run_report.json";

pub async fn exposure() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into());
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&report_path)?)?;
    let universe: Value = serde_json::from_str(&std::fs::read_to_string(UNIVERSE)?)?;
    let category_by_symbol = category_map(&universe);
    let positions = report
        .get("positions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut rows = Vec::new();
    let mut groups: BTreeMap<String, (Decimal, Decimal, usize)> = BTreeMap::new();
    let mut largest = json!({ "symbol": null, "weight_pct": "0", "value_usd": "0" });
    let mut largest_weight = Decimal::ZERO;
    let mut weights_desc = Vec::new();
    let mut categorized = 0usize;

    for position in positions {
        let symbol = position
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN")
            .to_string();
        let value_usd = decimal_field(&position, "value_usd");
        let weight_pct = decimal_field(&position, "weight_pct");
        let category = category_by_symbol
            .get(&symbol)
            .cloned()
            .unwrap_or_else(|| "uncategorized".to_string());
        if category != "uncategorized" {
            categorized += 1;
        }
        let entry = groups
            .entry(category.clone())
            .or_insert((Decimal::ZERO, Decimal::ZERO, 0));
        entry.0 += value_usd;
        entry.1 += weight_pct;
        entry.2 += 1;

        if weight_pct > largest_weight {
            largest_weight = weight_pct;
            largest = json!({
                "symbol": symbol,
                "category": category,
                "weight_pct": weight_pct.round_dp(2).to_string(),
                "value_usd": value_usd.round_dp(2).to_string()
            });
        }
        weights_desc.push(weight_pct);
        rows.push(json!({
            "symbol": symbol,
            "category": category,
            "value_usd": value_usd.round_dp(2).to_string(),
            "weight_pct": weight_pct.round_dp(2).to_string()
        }));
    }

    weights_desc.sort_by(|a, b| b.cmp(a));
    let top3_weight_pct: Decimal = weights_desc.iter().take(3).copied().sum();
    let stable_weight_pct = groups
        .get("stable")
        .map(|(_, weight, _)| *weight)
        .unwrap_or(Decimal::ZERO);
    let total_weight_pct: Decimal = weights_desc.iter().copied().sum();
    let risk_weight_pct = (total_weight_pct - stable_weight_pct).max(Decimal::ZERO);
    let status = if largest_weight > Decimal::from(25) || top3_weight_pct > Decimal::from(70) {
        "concentrated"
    } else if stable_weight_pct < Decimal::from(5) {
        "low_reserve"
    } else {
        "balanced"
    };

    let categories: Vec<Value> = groups
        .into_iter()
        .map(|(category, (value_usd, weight_pct, positions))| {
            json!({
                "category": category,
                "value_usd": value_usd.round_dp(2).to_string(),
                "weight_pct": weight_pct.round_dp(2).to_string(),
                "positions": positions
            })
        })
        .collect();

    Ok(json!({
        "status": status,
        "report_path": report_path,
        "universe_path": UNIVERSE,
        "nav_usd": report.get("nav_usd").cloned().unwrap_or(json!(null)),
        "positions": rows,
        "categories": categories,
        "summary": {
            "position_count": weights_desc.len(),
            "categorized_positions": categorized,
            "largest_position": largest,
            "top3_weight_pct": top3_weight_pct.round_dp(2).to_string(),
            "stable_weight_pct": stable_weight_pct.round_dp(2).to_string(),
            "risk_weight_pct": risk_weight_pct.round_dp(2).to_string()
        }
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

fn decimal_field(value: &Value, field: &str) -> Decimal {
    value
        .get(field)
        .and_then(Value::as_str)
        .and_then(|raw| Decimal::from_str(raw).ok())
        .unwrap_or(Decimal::ZERO)
}
