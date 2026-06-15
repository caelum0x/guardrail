//! Assets overview endpoint.
//!
//! Builds a fresh market snapshot from the mock data source over the eligible
//! universe and returns a normalized per-asset view plus the current Fear &
//! Greed reading. Read-only and side-effect free.

use axum::Json;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";

pub async fn assets() -> Json<Value> {
    match build().await {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

async fn build() -> anyhow::Result<Value> {
    let universe = market_data::Universe::load(UNIVERSE)?;
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await?;

    let fear_greed = snapshot.fear_greed.as_ref().map(|fg| {
        json!({
            "value": fg.value,
            "classification": fg.classification,
        })
    });

    let assets: Vec<Value> = snapshot
        .assets
        .iter()
        .map(|a| {
            json!({
                "symbol": a.asset.symbol,
                "price_usd": a.price_usd.to_string(),
                "ret_24h": a.ret_24h.map(|d| d.to_string()),
                "volume_24h_usd": a.volume_24h_usd.to_string(),
                "liquidity_usd": a.liquidity_usd.map(|d| d.to_string()),
                "safety_score": a.safety_score,
                "category": a.asset.category,
            })
        })
        .collect();

    Ok(json!({
        "fear_greed": fear_greed,
        "assets": assets,
    }))
}
