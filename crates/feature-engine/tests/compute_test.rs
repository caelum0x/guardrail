//! Integration tests for the feature engine: scores are normalized into 0..1
//! and momentum responds monotonically to higher return inputs.

use common::{Asset, AssetCategory, Decimal};
use feature_engine::FeatureEngine;
use market_data::{AssetMarketState, MarketSnapshot};

fn asset(symbol: &str, category: AssetCategory) -> Asset {
    Asset {
        symbol: symbol.to_string(),
        cmc_id: 1,
        chain_id: 56,
        contract_address: "0x0000000000000000000000000000000000000000".to_string(),
        decimals: 18,
        category,
    }
}

/// A reasonable non-stable asset state with a configurable 1h/24h return.
fn state_with_returns(symbol: &str, ret_1h: i64, ret_24h: i64) -> AssetMarketState {
    AssetMarketState {
        asset: asset(symbol, AssetCategory::Core),
        price_usd: Decimal::from(100),
        volume_24h_usd: Decimal::from(10_000_000),
        market_cap_usd: Some(Decimal::from(500_000_000)),
        liquidity_usd: Some(Decimal::from(5_000_000)),
        ret_1h: Some(Decimal::from(ret_1h)),
        ret_24h: Some(Decimal::from(ret_24h)),
        volatility_1h: Some(Decimal::from(2)),
        safety_score: 80,
        security_flags: vec![],
    }
}

fn snapshot(assets: Vec<AssetMarketState>) -> MarketSnapshot {
    MarketSnapshot {
        timestamp_ms: 0,
        assets,
        fear_greed: None,
        global_market: None,
    }
}

#[test]
fn compute_produces_scores_within_unit_range() {
    let snap = snapshot(vec![
        state_with_returns("AAA", 3, 10),
        state_with_returns("BBB", -2, -5),
    ]);
    let features = FeatureEngine::new().compute(&snap);
    assert_eq!(features.len(), 2);

    for f in &features {
        for (label, value) in [
            ("momentum", f.momentum_score),
            ("volume", f.volume_acceleration_score),
            ("volatility", f.volatility_score),
            ("liquidity", f.liquidity_score),
            ("sentiment", f.sentiment_score),
            ("execution_quality", f.execution_quality_score),
            ("risk_penalty", f.risk_penalty),
        ] {
            assert!(
                (0.0..=1.0).contains(&value),
                "{} score {} for {} out of 0..1",
                label,
                value,
                f.symbol
            );
        }
    }
}

#[test]
fn higher_momentum_input_yields_higher_momentum_score() {
    let snap = snapshot(vec![
        state_with_returns("LOW", -3, -8),
        state_with_returns("HIGH", 5, 15),
    ]);
    let features = FeatureEngine::new().compute(&snap);

    let low = features.iter().find(|f| f.symbol == "LOW").unwrap();
    let high = features.iter().find(|f| f.symbol == "HIGH").unwrap();

    assert!(
        high.momentum_score > low.momentum_score,
        "expected higher momentum input to score higher: high={} low={}",
        high.momentum_score,
        low.momentum_score
    );
}

#[test]
fn compute_excludes_stable_assets() {
    let snap = snapshot(vec![
        state_with_returns("CORE", 2, 4),
        AssetMarketState {
            asset: asset("USDT", AssetCategory::Stable),
            ..state_with_returns("USDT", 0, 0)
        },
    ]);
    let features = FeatureEngine::new().compute(&snap);
    assert_eq!(features.len(), 1);
    assert_eq!(features[0].symbol, "CORE");
}
