//! Liquidity constraints endpoint.
//!
//! Computes per-asset capacity and utilization against configured pool-usage
//! limits using the current CMC market snapshot. Read-only.

use axum::Json;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use serde_json::{json, Value};

const POLICY: &str = "configs/liquidity/liquidity_policy.json";

pub async fn liquidity() -> Json<Value> {
    match build().await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build() -> anyhow::Result<Value> {
    let policy: Value = serde_json::from_str(&std::fs::read_to_string(POLICY)?)?;
    let universe_path = policy
        .get("universe_path")
        .and_then(Value::as_str)
        .unwrap_or("configs/eligible_assets.bsc.json");
    let max_usage = decimal_config(&policy, "max_pool_usage_pct", Decimal::new(5, 1));
    let warning_usage = decimal_config(&policy, "warning_pool_usage_pct", Decimal::new(35, 2));
    let min_liquidity = decimal_config(&policy, "min_liquidity_usd", Decimal::from(500_000));
    let notional = decimal_config(&policy, "default_order_notional_usd", Decimal::from(1000));

    let universe = market_data::Universe::load(universe_path)?;
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;
    let mut rows = Vec::new();
    let mut blocking = 0usize;
    let mut watch = 0usize;

    for asset in snapshot.assets {
        if asset.asset.category.is_stable() {
            continue;
        }
        let liquidity = asset.liquidity_usd.unwrap_or(Decimal::ZERO);
        let capacity = liquidity * max_usage / Decimal::from(100);
        let usage = if liquidity > Decimal::ZERO {
            notional / liquidity * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        let headroom = (capacity - notional).max(Decimal::ZERO);
        let status = if liquidity < min_liquidity || usage > max_usage {
            blocking += 1;
            "blocking"
        } else if usage >= warning_usage {
            watch += 1;
            "watch"
        } else {
            "ok"
        };
        rows.push(json!({
            "symbol": asset.asset.symbol,
            "category": format!("{:?}", asset.asset.category).to_ascii_lowercase(),
            "status": status,
            "liquidity_usd": liquidity.round_dp(2).to_string(),
            "capacity_usd": capacity.round_dp(2).to_string(),
            "order_notional_usd": notional.round_dp(2).to_string(),
            "pool_usage_pct": usage.round_dp(4).to_string(),
            "headroom_usd": headroom.round_dp(2).to_string(),
            "ret_24h": asset.ret_24h.map(|value| value.round_dp(2).to_string()),
            "safety_score": asset.safety_score
        }));
    }
    rows.sort_by(|a, b| decimal_field(a, "headroom_usd").cmp(&decimal_field(b, "headroom_usd")));

    Ok(json!({
        "policy_path": POLICY,
        "universe_path": universe_path,
        "thresholds": {
            "max_pool_usage_pct": max_usage.to_string(),
            "warning_pool_usage_pct": warning_usage.to_string(),
            "min_liquidity_usd": min_liquidity.to_string(),
            "default_order_notional_usd": notional.to_string()
        },
        "summary": {
            "assets": rows.len(),
            "blocking": blocking,
            "watch": watch,
            "ok": rows.len().saturating_sub(blocking + watch)
        },
        "assets": rows
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
                .and_then(|value| value.parse::<Decimal>().ok())
        })
        .unwrap_or(default)
}

fn decimal_field(value: &Value, key: &str) -> Decimal {
    value
        .get(key)
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<Decimal>().ok())
        .unwrap_or(Decimal::ZERO)
}
