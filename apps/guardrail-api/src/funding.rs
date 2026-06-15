//! Derivatives-style funding proxy endpoint.
//!
//! Builds a fresh market snapshot from the mock data source over the eligible
//! universe and computes a synthetic per-hour funding-rate proxy for each
//! non-stable asset. This approximates perpetual-swap funding pressure for use
//! in regime rotation. Read-only and side-effect free.

use axum::Json;
use common::decimal::to_f64;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";

/// Bounds for the synthetic funding-rate proxy (per hour).
const FUNDING_MIN: f64 = -1.0;
const FUNDING_MAX: f64 = 1.0;
/// Baseline volatility (percent) at which the volatility term is neutral.
const VOL_BASELINE: f64 = 3.0;
/// Weight applied to the volatility deviation term.
const VOL_WEIGHT: f64 = 0.01;
/// Number of hours used to spread the 24h return across an hourly proxy.
const HOURS_PER_DAY: f64 = 24.0;

async fn build() -> anyhow::Result<Value> {
    let universe = market_data::Universe::load(UNIVERSE)?;
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;

    let assets: Vec<Value> = snapshot
        .assets
        .iter()
        .filter(|a| !a.asset.category.is_stable())
        .map(|a| {
            let ret_24h_pct = a.ret_24h.map(to_f64).unwrap_or(0.0);
            let volatility_1h = a.volatility_1h.map(to_f64).unwrap_or(0.0);
            let proxy = funding_rate_proxy(ret_24h_pct, volatility_1h);

            json!({
                "symbol": a.asset.symbol,
                "price_usd": a.price_usd.to_string(),
                "ret_24h": a.ret_24h.map(|d| d.to_string()),
                "funding_rate_proxy": proxy.to_string(),
            })
        })
        .collect();

    Ok(json!({ "assets": assets }))
}

/// Synthetic per-hour funding-rate proxy, clamped to [-1.0, 1.0].
fn funding_rate_proxy(ret_24h_pct: f64, volatility_1h: f64) -> f64 {
    let raw = ret_24h_pct / HOURS_PER_DAY + (volatility_1h - VOL_BASELINE) * VOL_WEIGHT;
    raw.clamp(FUNDING_MIN, FUNDING_MAX)
}

pub async fn funding() -> Json<Value> {
    match build().await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}
