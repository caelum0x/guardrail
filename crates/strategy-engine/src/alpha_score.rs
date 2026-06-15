//! Alpha scoring: blend per-asset features into a single 0..1 conviction.

use crate::strategy_config::StrategyConfig;
use feature_engine::AssetFeatures;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredAsset {
    pub symbol: String,
    pub score: f64,
    pub risk_penalty: f64,
}

/// Compute blended alpha scores, sorted descending by score.
pub fn compute(features: &[AssetFeatures], cfg: &StrategyConfig) -> Vec<ScoredAsset> {
    let w = &cfg.weights;
    let weight_sum =
        w.momentum + w.volume + w.volatility + w.liquidity + w.sentiment + w.execution_quality;

    let mut scored: Vec<ScoredAsset> = features
        .iter()
        .map(|f| {
            let raw = w.momentum * f.momentum_score
                + w.volume * f.volume_acceleration_score
                + w.volatility * f.volatility_score
                + w.liquidity * f.liquidity_score
                + w.sentiment * f.sentiment_score
                + w.execution_quality * f.execution_quality_score;
            let normalized = if weight_sum > 0.0 {
                raw / weight_sum
            } else {
                0.0
            };
            // Apply the security penalty as a multiplicative haircut.
            let score = (normalized * (1.0 - f.risk_penalty)).clamp(0.0, 1.0);
            ScoredAsset {
                symbol: f.symbol.clone(),
                score,
                risk_penalty: f.risk_penalty,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored
}
