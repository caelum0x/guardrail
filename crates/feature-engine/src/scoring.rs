//! The feature engine: assembles per-asset features from a snapshot.

use crate::{execution_quality, liquidity, momentum, risk_penalty, sentiment, volatility, volume};
use market_data::MarketSnapshot;
use serde::{Deserialize, Serialize};

/// Normalized 0..1 feature scores for a single asset (risk_penalty is 0..1 bad).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFeatures {
    pub symbol: String,
    pub momentum_score: f64,
    pub volume_acceleration_score: f64,
    pub volatility_score: f64,
    pub liquidity_score: f64,
    pub sentiment_score: f64,
    pub execution_quality_score: f64,
    pub risk_penalty: f64,
}

#[derive(Default)]
pub struct FeatureEngine;

impl FeatureEngine {
    pub fn new() -> Self {
        FeatureEngine
    }

    /// Compute features for every non-stable asset in the snapshot.
    pub fn compute(&self, snap: &MarketSnapshot) -> Vec<AssetFeatures> {
        let sentiment_score = snap
            .fear_greed
            .as_ref()
            .map(|f| sentiment::score_from_fear_greed(f.value))
            .unwrap_or(0.5);

        snap.assets
            .iter()
            .filter(|a| !a.asset.category.is_stable())
            .map(|a| AssetFeatures {
                symbol: a.asset.symbol.clone(),
                momentum_score: momentum::score(a),
                volume_acceleration_score: volume::score(a),
                volatility_score: volatility::score(a),
                liquidity_score: liquidity::score(a),
                sentiment_score,
                execution_quality_score: execution_quality::score(a),
                risk_penalty: risk_penalty::penalty(a),
            })
            .collect()
    }
}
