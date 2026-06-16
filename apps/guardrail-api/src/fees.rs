//! Swap-cost endpoint: `GET /fees`.
//!
//! Estimates the all-in cost of a swap using the real `fee-model` crate (gas +
//! price-impact/slippage + protocol fee). Read-only, pure. Query params (all
//! optional, sensible defaults):
//! `notional_usd, quantity, side(buy|sell), gas_units, gas_price_gwei,
//!  native_usd, pool_liquidity_usd, linear_slippage_bps, protocol_fee_bps`.

use std::str::FromStr;

use axum::extract::Query;
use axum::Json;
use fee_model::{SwapCostModel, SwapSide};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct FeeQuery {
    notional_usd: Option<String>,
    quantity: Option<String>,
    side: Option<String>,
    gas_units: Option<String>,
    gas_price_gwei: Option<String>,
    native_usd: Option<String>,
    pool_liquidity_usd: Option<String>,
    linear_slippage_bps: Option<String>,
    protocol_fee_bps: Option<String>,
}

/// Parse an optional decimal string, falling back to `default`.
fn dec(opt: &Option<String>, default: &str) -> Decimal {
    opt.as_deref()
        .and_then(|s| Decimal::from_str(s.trim()).ok())
        .unwrap_or_else(|| Decimal::from_str(default).expect("valid default"))
}

pub async fn fees(Query(q): Query<FeeQuery>) -> Json<Value> {
    let side = match q.side.as_deref() {
        Some("sell") | Some("Sell") => SwapSide::Sell,
        _ => SwapSide::Buy,
    };

    let model = SwapCostModel::builder()
        .notional_usd(dec(&q.notional_usd, "10000"))
        .quantity(dec(&q.quantity, "5"))
        .side(side)
        .gas(
            dec(&q.gas_units, "150000"),
            dec(&q.gas_price_gwei, "1"),
            dec(&q.native_usd, "600"),
        )
        .pool_liquidity_usd(dec(&q.pool_liquidity_usd, "2000000"))
        .linear_slippage_bps(dec(&q.linear_slippage_bps, "5"))
        .protocol_fee_bps(dec(&q.protocol_fee_bps, "30"))
        .build();

    let breakdown = model.estimate();
    Json(json!({
        "side": if matches!(side, SwapSide::Sell) { "sell" } else { "buy" },
        "breakdown": breakdown,
    }))
}
