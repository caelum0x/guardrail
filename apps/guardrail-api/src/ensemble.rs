//! Regime-routed skill-ensemble view.
//!
//! The ensemble is a meta-allocator that blends the four Track-2 strategy
//! Skills by market regime: for the currently classified regime, each Skill
//! contributes its example target portfolio with the weight listed below, and
//! the ensemble takes the weighted average of the per-Skill target weights.
//!
//! This endpoint surfaces the static weight table (a constant mirror of
//! `skills/ensemble.json`) together with the *current* classified regime, so an
//! operator can see which Skills are leading right now. Read-only and
//! side-effect free; if the live snapshot cannot be built the regime degrades
//! to `null` while the full weight table is still returned.

use axum::Json;
use serde_json::{json, Value};

const UNIVERSE: &str = "configs/eligible_assets.bsc.json";
const RESERVE_SYMBOL: &str = "USDT";
const MAX_RISK_ALLOCATION_PCT: f64 = 100.0;

/// The four ensemble Skills, in display order, with `(id, label)`.
const SKILLS: [(&str, &str); 4] = [
    ("cmc-regime-routed-alpha", "general regime-routed alpha"),
    ("funding-rate-carry", "funding-rate / basis carry"),
    ("mean-reversion-chop", "mean-reversion / range-fade"),
    ("trend-breakout-momentum", "trend / breakout momentum"),
];

/// Per-regime Skill weights, mirroring `skills/ensemble.json`.
/// Order of the inner array matches [`SKILLS`].
const REGIME_WEIGHTS: [(&str, [f64; 4]); 4] = [
    ("risk_on", [0.35, 0.25, 0.05, 0.35]),
    ("risk_off", [0.45, 0.35, 0.10, 0.10]),
    ("chop", [0.30, 0.12, 0.50, 0.08]),
    ("breakout", [0.30, 0.15, 0.05, 0.50]),
];

pub async fn ensemble() -> Json<Value> {
    let current_regime = classify_current().await;
    let active_weights = current_regime.as_deref().and_then(weights_for);

    Json(json!({
        "name": "guardrail-regime-ensemble",
        "version": "1.0.0",
        "reserve_symbol": RESERVE_SYMBOL,
        "max_risk_allocation_pct": MAX_RISK_ALLOCATION_PCT,
        "current_regime": current_regime,
        "active_weights": active_weights,
        "skills": SKILLS
            .iter()
            .map(|(id, label)| json!({ "id": id, "label": label }))
            .collect::<Vec<_>>(),
        "regimes": REGIME_WEIGHTS
            .iter()
            .map(|(regime, weights)| {
                json!({
                    "regime": regime,
                    "weights": weights_object(weights),
                })
            })
            .collect::<Vec<_>>(),
    }))
}

/// Builds the per-Skill weight object for a single regime row.
fn weights_object(weights: &[f64; 4]) -> Value {
    let mut map = serde_json::Map::new();
    for ((id, _label), weight) in SKILLS.iter().zip(weights.iter()) {
        map.insert((*id).to_string(), json!(weight));
    }
    Value::Object(map)
}

/// Returns the weight object for the named regime, if recognised.
fn weights_for(regime: &str) -> Option<Value> {
    REGIME_WEIGHTS
        .iter()
        .find(|(name, _)| *name == regime)
        .map(|(_, weights)| weights_object(weights))
}

/// Classifies the current market regime from a fresh mock snapshot, mirroring
/// the `/regime` endpoint. Returns `None` if the snapshot cannot be built so
/// the endpoint never fails on data-source errors.
async fn classify_current() -> Option<String> {
    let universe = market_data::Universe::load(UNIVERSE).ok()?;
    let source = cmc_client::MockCmcClient::new();
    let snapshot = market_data::SnapshotBuilder::new(&source, &universe)
        .build()
        .await
        .ok()?;
    let inputs = market_data::RegimeInputs::from_snapshot(&snapshot);
    Some(
        strategy_engine::regime::classify(&inputs)
            .as_str()
            .to_string(),
    )
}
