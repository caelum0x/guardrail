//! Execution cost preview endpoint.
//!
//! Estimates BSC gas and slippage drag for configured TWAK routes. Read-only:
//! no risk approval, quote submission, or swap execution happens here.

use axum::extract::Query;
use axum::Json;
use common::{Decimal, OrderIntent, OrderSide};
use rust_decimal::prelude::FromPrimitive;
use serde::Deserialize;
use serde_json::{json, Value};
use twak_client::{MockTwakClient, TwakExecutor};

const COST_CONFIG: &str = "configs/costs/bsc_execution_costs.json";

#[derive(Debug, Deserialize)]
pub struct CostParams {
    /// Optional notional applied to every configured route.
    pub amount_usd: Option<f64>,
}

pub async fn costs(Query(params): Query<CostParams>) -> Json<Value> {
    match build(&params).await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build(params: &CostParams) -> anyhow::Result<Value> {
    let config: Value = serde_json::from_str(&std::fs::read_to_string(COST_CONFIG)?)?;
    let native_price = decimal_config(&config, "native_price_usd", Decimal::from(610));
    let gas_price_gwei = decimal_config(&config, "gas_price_gwei", Decimal::from(3));
    let quote_gas = decimal_config(&config, "quote_gas_units", Decimal::from(45_000));
    let swap_gas = decimal_config(&config, "swap_gas_units", Decimal::from(210_000));
    let approval_gas = decimal_config(&config, "approval_gas_units", Decimal::from(65_000));
    let approval_required = config
        .get("approval_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let default_amount = decimal_config(&config, "default_order_notional_usd", Decimal::from(1000));
    let amount = params
        .amount_usd
        .and_then(Decimal::from_f64)
        .unwrap_or(default_amount)
        .clamp(Decimal::from(1), Decimal::from(100_000));
    let per_route_gas_units = quote_gas
        + swap_gas
        + if approval_required {
            approval_gas
        } else {
            Decimal::ZERO
        };
    let gas_usd = gas_cost_usd(per_route_gas_units, gas_price_gwei, native_price);
    let client = MockTwakClient::new();
    let mut rows = Vec::new();
    let mut total_gas_usd = Decimal::ZERO;
    let mut total_slippage_usd = Decimal::ZERO;

    for route in config
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let from = route.get("from").and_then(Value::as_str).unwrap_or("USDT");
        let to = route.get("to").and_then(Value::as_str).unwrap_or("WBNB");
        let side = match route.get("side").and_then(Value::as_str) {
            Some("sell") => OrderSide::Sell,
            _ => OrderSide::Buy,
        };
        let intent = OrderIntent::new(side, from, to, amount, format!("cost preview {from}->{to}"));
        let quote = client.quote_swap(&intent).await?;
        let slippage_usd = (amount - quote.summary.expected_out_usd).max(Decimal::ZERO);
        let all_in_cost = gas_usd + slippage_usd;
        total_gas_usd += gas_usd;
        total_slippage_usd += slippage_usd;
        rows.push(json!({
            "route": format!("{from}->{to}"),
            "side": side,
            "amount_usd": amount.round_dp(2).to_string(),
            "gas_units": per_route_gas_units.round_dp(0).to_string(),
            "gas_usd": gas_usd.round_dp(4).to_string(),
            "slippage_usd": slippage_usd.round_dp(4).to_string(),
            "all_in_cost_usd": all_in_cost.round_dp(4).to_string(),
            "all_in_cost_bps": bps(all_in_cost, amount).round_dp(2).to_string(),
            "price_impact_pct": quote.summary.price_impact_pct.to_string(),
            "slippage_pct": quote.summary.slippage_pct.to_string(),
            "expected_out_usd": quote.summary.expected_out_usd.to_string()
        }));
    }

    let total_all_in = total_gas_usd + total_slippage_usd;
    Ok(json!({
        "preview_only": true,
        "config_path": COST_CONFIG,
        "chain": config.get("chain").cloned().unwrap_or(json!("bsc")),
        "native_symbol": config.get("native_symbol").cloned().unwrap_or(json!("BNB")),
        "assumptions": {
            "native_price_usd": native_price.to_string(),
            "gas_price_gwei": gas_price_gwei.to_string(),
            "quote_gas_units": quote_gas.to_string(),
            "swap_gas_units": swap_gas.to_string(),
            "approval_gas_units": approval_gas.to_string(),
            "approval_required": approval_required
        },
        "summary": {
            "routes": rows.len(),
            "amount_usd": amount.round_dp(2).to_string(),
            "total_gas_usd": total_gas_usd.round_dp(4).to_string(),
            "total_slippage_usd": total_slippage_usd.round_dp(4).to_string(),
            "total_all_in_cost_usd": total_all_in.round_dp(4).to_string(),
            "average_cost_bps": if rows.is_empty() {
                "0".to_string()
            } else {
                bps(total_all_in, amount * Decimal::from(rows.len() as i64)).round_dp(2).to_string()
            }
        },
        "routes": rows
    }))
}

fn gas_cost_usd(gas_units: Decimal, gas_price_gwei: Decimal, native_price: Decimal) -> Decimal {
    let native_amount = gas_units * gas_price_gwei / Decimal::from(1_000_000_000u64);
    native_amount * native_price
}

fn bps(cost: Decimal, notional: Decimal) -> Decimal {
    if notional <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        cost / notional * Decimal::from(10_000)
    }
}

fn decimal_config(config: &Value, key: &str, default: Decimal) -> Decimal {
    config
        .get(key)
        .and_then(Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| config.get(key).and_then(Value::as_i64).map(Decimal::from))
        .or_else(|| {
            config
                .get(key)
                .and_then(Value::as_u64)
                .and_then(Decimal::from_u64)
        })
        .or_else(|| {
            config
                .get(key)
                .and_then(Value::as_str)
                .and_then(|value| value.parse::<Decimal>().ok())
        })
        .unwrap_or(default)
}
