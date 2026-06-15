//! Market regime detail endpoint.
//!
//! Builds a fresh market snapshot from the mock data source over the eligible
//! universe, derives the compact regime inputs, and classifies the current
//! market regime. Read-only and side-effect free.

use axum::Json;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";

pub async fn regime() -> Json<Value> {
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

    let inputs = market_data::RegimeInputs::from_snapshot(&snapshot);
    let regime = strategy_engine::regime::classify(&inputs);

    Ok(json!({
        "regime": regime.as_str(),
        "exposure_multiplier": regime.exposure_multiplier().to_string(),
        "inputs": {
            "fear_greed": inputs.fear_greed,
            "breadth_pct": inputs.breadth_pct.to_string(),
            "btc_dominance_pct": inputs.btc_dominance_pct.to_string(),
            "median_24h_return": inputs.median_24h_return.to_string(),
        },
    }))
}
