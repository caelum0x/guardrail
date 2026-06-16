//! Portfolio concentration-risk endpoint: `GET /portfolio/risk`.
//!
//! Computes concentration metrics over the latest run report's positions —
//! Herfindahl-Hirschman Index (HHI), effective number of positions, largest
//! position, invested vs. stable-reserve split. Read-only; pure arithmetic over
//! data the agent already wrote. Complements `/exposure` (category view) and
//! `/risk` (drawdown/kill-switch) with a concentration view.

use axum::Json;
use serde_json::{json, Value};

const DEFAULT_REPORT: &str = "data/run_report.json";

pub async fn portfolio_risk() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn round(v: f64, dp: i32) -> f64 {
    let f = 10f64.powi(dp);
    (v * f).round() / f
}

fn build() -> anyhow::Result<Value> {
    let path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into());
    let report: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let positions = report
        .get("positions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut sum_pct = 0.0;
    let mut hhi = 0.0; // over weight fractions, in [0, 1]
    let mut max_weight = 0.0;
    let mut max_symbol = String::new();

    for p in &positions {
        let w_pct = p
            .get("weight_pct")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        sum_pct += w_pct;
        let frac = w_pct / 100.0;
        hhi += frac * frac;
        if w_pct > max_weight {
            max_weight = w_pct;
            max_symbol = p
                .get("symbol")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
        }
    }

    let effective_n = if hhi > 0.0 { 1.0 / hhi } else { 0.0 };
    let reserve_pct = (100.0 - sum_pct).max(0.0);
    // HHI thresholds (fraction-based): <0.15 diversified, <0.30 moderate, else concentrated.
    let concentration = if positions.is_empty() {
        "empty"
    } else if hhi <= 0.15 {
        "diversified"
    } else if hhi <= 0.30 {
        "moderate"
    } else {
        "concentrated"
    };

    Ok(json!({
        "positions": positions.len(),
        "invested_pct": round(sum_pct, 2),
        "stable_reserve_pct": round(reserve_pct, 2),
        "max_position": { "symbol": max_symbol, "weight_pct": round(max_weight, 2) },
        "hhi": round(hhi, 4),
        "effective_n": round(effective_n, 2),
        "concentration": concentration,
    }))
}
