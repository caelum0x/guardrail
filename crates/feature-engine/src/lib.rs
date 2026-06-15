//! Feature computation.
//!
//! Turns an [`AssetMarketState`] into normalized 0..1 scores the strategy
//! engine blends into an alpha score. All scoring math runs in `f64`; money
//! never does.

pub mod execution_quality;
pub mod liquidity;
pub mod momentum;
pub mod normalization;
pub mod risk_penalty;
pub mod scoring;
pub mod sentiment;
pub mod volatility;
pub mod volume;

pub use scoring::{AssetFeatures, FeatureEngine};
