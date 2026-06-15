//! Operator watchlist endpoint.
//!
//! Ranks enabled assets by current attention needs using market snapshot facts:
//! return move, volatility, liquidity, safety score, and security flags.

use axum::extract::Query;
use axum::Json;
use common::decimal::to_f64;
use serde::Deserialize;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";

#[derive(Debug, Deserialize)]
pub struct WatchlistParams {
    /// Number of rows to return (default 12, max 50).
    pub limit: Option<usize>,
}

pub async fn watchlist(Query(params): Query<WatchlistParams>) -> Json<Value> {
    match build(params.limit.unwrap_or(12).clamp(1, 50)).await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build(limit: usize) -> anyhow::Result<Value> {
    let universe = market_data::Universe::load(UNIVERSE)?;
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;
    let mut rows = Vec::new();

    for asset in &snapshot.assets {
        if asset.asset.category.is_stable() {
            continue;
        }
        let ret_24h = asset.ret_24h.map(to_f64).unwrap_or(0.0);
        let volatility = asset.volatility_1h.map(to_f64).unwrap_or(0.0);
        let liquidity = asset.liquidity_usd.map(to_f64).unwrap_or(0.0);
        let safety_penalty = ((100_i64 - asset.safety_score as i64).max(0) as f64) / 10.0;
        let liquidity_penalty = if liquidity > 0.0 && liquidity < 500_000.0 {
            8.0
        } else {
            0.0
        };
        let flag_penalty = asset.security_flags.len() as f64 * 6.0;
        let attention_score =
            ret_24h.abs() + (volatility * 2.0) + safety_penalty + liquidity_penalty + flag_penalty;
        let mut reasons = Vec::new();
        if ret_24h.abs() >= 5.0 {
            reasons.push(format!("24h move {:.2}%", ret_24h));
        }
        if volatility >= 4.0 {
            reasons.push(format!("1h volatility {:.2}%", volatility));
        }
        if asset.safety_score < 70 {
            reasons.push(format!("safety score {}", asset.safety_score));
        }
        if liquidity_penalty > 0.0 {
            reasons.push(format!("liquidity ${:.0}", liquidity));
        }
        for flag in &asset.security_flags {
            reasons.push(format!("flag {flag}"));
        }
        if reasons.is_empty() {
            reasons.push("normal market facts".to_string());
        }
        let status = if attention_score >= 25.0 {
            "critical"
        } else if attention_score >= 12.0 {
            "watch"
        } else {
            "normal"
        };
        rows.push(json!({
            "symbol": asset.asset.symbol,
            "category": format!("{:?}", asset.asset.category).to_ascii_lowercase(),
            "status": status,
            "attention_score": format!("{attention_score:.2}"),
            "price_usd": asset.price_usd.round_dp(6).to_string(),
            "ret_24h": asset.ret_24h.map(|value| value.round_dp(2).to_string()),
            "volatility_1h": asset.volatility_1h.map(|value| value.round_dp(2).to_string()),
            "liquidity_usd": asset.liquidity_usd.map(|value| value.round_dp(2).to_string()),
            "safety_score": asset.safety_score,
            "security_flags": asset.security_flags,
            "reasons": reasons
        }));
    }

    rows.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(limit);
    let critical = rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("critical"))
        .count();
    let watch = rows
        .iter()
        .filter(|row| row.get("status").and_then(Value::as_str) == Some("watch"))
        .count();

    Ok(json!({
        "limit": limit,
        "universe_path": UNIVERSE,
        "fear_greed": snapshot.fear_greed.map(|fg| json!({
            "value": fg.value,
            "classification": fg.classification
        })),
        "counts": {
            "critical": critical,
            "watch": watch,
            "total": rows.len()
        },
        "assets": rows
    }))
}

fn score(row: &Value) -> f64 {
    row.get("attention_score")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
}
