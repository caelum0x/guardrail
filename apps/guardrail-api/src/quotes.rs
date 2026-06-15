//! TWAK quote preview endpoint.
//!
//! Computes deterministic quote previews through the same TWAK quote interface
//! used by the execution path. Read-only: no approval or swap is submitted.

use axum::extract::Query;
use axum::Json;
use common::{Decimal, OrderIntent, OrderSide};
use rust_decimal::prelude::FromPrimitive;
use serde::Deserialize;
use serde_json::{json, Value};
use twak_client::{MockTwakClient, TwakExecutor};

const DEFAULT_AMOUNT_USD: f64 = 1_000.0;

#[derive(Debug, Deserialize)]
pub struct QuotesParams {
    /// Notional used for every sample route.
    pub amount_usd: Option<f64>,
}

pub async fn quotes(Query(params): Query<QuotesParams>) -> Json<Value> {
    match build(&params).await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build(params: &QuotesParams) -> anyhow::Result<Value> {
    let amount = Decimal::from_f64(
        params
            .amount_usd
            .unwrap_or(DEFAULT_AMOUNT_USD)
            .clamp(1.0, 100_000.0),
    )
    .unwrap_or_else(|| Decimal::from(1_000));
    let routes = [
        ("USDT", "WBNB", OrderSide::Buy),
        ("USDT", "ETH", OrderSide::Buy),
        ("USDT", "DOGE", OrderSide::Buy),
        ("WBNB", "USDT", OrderSide::Sell),
        ("ETH", "USDT", OrderSide::Sell),
    ];
    let client = MockTwakClient::new();
    let wallet = client.wallet_address().await?;
    let mut previews = Vec::new();

    for (from, to, side) in routes {
        let intent = OrderIntent::new(
            side,
            from,
            to,
            amount,
            format!("quote preview {from}->{to}"),
        );
        let quote = client.quote_swap(&intent).await?;
        let severity = if quote.summary.slippage_pct >= Decimal::new(100, 2) {
            "high"
        } else if quote.summary.slippage_pct >= Decimal::new(25, 2) {
            "watch"
        } else {
            "normal"
        };
        previews.push(json!({
            "route": format!("{from}->{to}"),
            "side": side,
            "from_symbol": from,
            "to_symbol": to,
            "amount_usd": amount.round_dp(2).to_string(),
            "route_id": quote.route_id,
            "expected_out_symbol": quote.expected_out_symbol,
            "expected_out_amount": quote.expected_out_amount.to_string(),
            "summary": {
                "expected_out_usd": quote.summary.expected_out_usd.to_string(),
                "price_impact_pct": quote.summary.price_impact_pct.to_string(),
                "slippage_pct": quote.summary.slippage_pct.to_string(),
                "liquidity_usd": quote.summary.liquidity_usd.to_string(),
            },
            "severity": severity,
        }));
    }

    Ok(json!({
        "preview_only": true,
        "wallet_address": wallet.0,
        "amount_usd": amount.round_dp(2).to_string(),
        "routes": previews,
        "execution_note": "Quotes are informational. Swaps require risk approval and TWAK execution."
    }))
}
