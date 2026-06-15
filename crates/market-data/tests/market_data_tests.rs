//! Integration tests for the market-data crate.
//!
//! These exercise the public API end-to-end against the deterministic
//! `MockCmcClient`, plus the pure helper functions for liquidity, security,
//! validation, universe membership, and regime-input derivation.

use cmc_client::{FearGreedSnapshot, MockCmcClient};
use common::{Asset, AssetCategory, Decimal, EligibleAsset};
use market_data::snapshot::{AssetMarketState, GlobalMarketState};
use market_data::{
    liquidity, security, validator, MarketSnapshot, RegimeInputs, SnapshotBuilder, Universe,
};
use rust_decimal_macros::dec;

/// Build an `EligibleAsset` with the fields the universe/snapshot paths read.
fn eligible(symbol: &str, cmc_id: u64, category: AssetCategory, enabled: bool) -> EligibleAsset {
    EligibleAsset {
        symbol: symbol.to_string(),
        cmc_id,
        chain_id: 56,
        contract_address: format!("0x{symbol}"),
        decimals: 18,
        category,
        enabled,
        min_liquidity_usd: Decimal::ZERO,
        min_volume_24h_usd: Decimal::ZERO,
    }
}

/// A universe with two enabled non-stables, one enabled stable, and one
/// disabled asset.
fn sample_universe() -> Universe {
    Universe::new(vec![
        eligible("USDT", 825, AssetCategory::Stable, true),
        eligible("WBNB", 1839, AssetCategory::Core, true),
        eligible("CAKE", 7186, AssetCategory::DeFi, true),
        eligible("SCAM", 9999, AssetCategory::Meme, false),
    ])
}

/// A plain `AssetMarketState` for the pure-helper tests.
fn asset_state(
    symbol: &str,
    safety_score: u32,
    flags: Vec<String>,
    liquidity_usd: Option<Decimal>,
) -> AssetMarketState {
    AssetMarketState {
        asset: Asset {
            symbol: symbol.to_string(),
            cmc_id: 1,
            chain_id: 56,
            contract_address: format!("0x{symbol}"),
            decimals: 18,
            category: AssetCategory::Core,
        },
        price_usd: dec!(10),
        volume_24h_usd: dec!(1_000_000),
        market_cap_usd: Some(dec!(50_000_000)),
        liquidity_usd,
        ret_1h: Some(dec!(1)),
        ret_24h: Some(dec!(2)),
        volatility_1h: Some(dec!(3)),
        safety_score,
        security_flags: flags,
    }
}

#[tokio::test]
async fn builds_snapshot_matching_enabled_universe() {
    let universe = sample_universe();
    let client = MockCmcClient::new();

    let snapshot = SnapshotBuilder::new(&client, &universe)
        .build()
        .await
        .expect("snapshot builds");

    // Only the three enabled assets appear; the disabled one does not.
    assert_eq!(snapshot.assets.len(), 3);
    let mut symbols: Vec<&str> = snapshot
        .assets
        .iter()
        .map(|a| a.asset.symbol.as_str())
        .collect();
    symbols.sort();
    assert_eq!(symbols, vec!["CAKE", "USDT", "WBNB"]);
    assert!(snapshot.get("SCAM").is_none());

    // Every asset has a positive price.
    for state in &snapshot.assets {
        assert!(
            state.price_usd > Decimal::ZERO,
            "{} price should be positive, got {}",
            state.asset.symbol,
            state.price_usd
        );
    }

    // Fear/greed sentiment is present and in range.
    let fg = snapshot.fear_greed.as_ref().expect("fear_greed present");
    assert!(fg.value <= 100);

    // Global market context was attached.
    assert!(snapshot.global_market.is_some());
}

#[tokio::test]
async fn built_snapshot_passes_validation() {
    let universe = sample_universe();
    let client = MockCmcClient::new();
    let snapshot = SnapshotBuilder::new(&client, &universe)
        .build()
        .await
        .expect("snapshot builds");

    let validity = validator::validate(&snapshot);
    assert!(validity.is_ok(), "expected Ok, got {validity:?}");
}

#[tokio::test]
async fn regime_inputs_derived_from_built_snapshot() {
    // Greedy sentiment so we can assert the propagated value.
    let universe = sample_universe();
    let client = MockCmcClient::with_fear_greed(70);
    let snapshot = SnapshotBuilder::new(&client, &universe)
        .build()
        .await
        .expect("snapshot builds");

    let inputs = RegimeInputs::from_snapshot(&snapshot);

    // Fear/greed flows straight through from the snapshot.
    assert_eq!(inputs.fear_greed, 70);

    // Breadth is a percentage in [0, 100]. The mock gives non-stables positive
    // 24h returns, so breadth should be 100% of the two non-stable assets.
    assert!(inputs.breadth_pct >= Decimal::ZERO);
    assert!(inputs.breadth_pct <= dec!(100));
    assert_eq!(inputs.breadth_pct, dec!(100));

    // BTC dominance came from the mock global market (52.30%).
    assert_eq!(inputs.btc_dominance_pct, dec!(52.30));
}

#[test]
fn regime_inputs_defaults_when_snapshot_lacks_context() {
    let snapshot = MarketSnapshot {
        timestamp_ms: 0,
        assets: vec![],
        fear_greed: None,
        global_market: None,
    };

    let inputs = RegimeInputs::from_snapshot(&snapshot);
    // Missing sentiment defaults to neutral 50; empty universe -> zero breadth.
    assert_eq!(inputs.fear_greed, 50);
    assert_eq!(inputs.breadth_pct, Decimal::ZERO);
    assert_eq!(inputs.btc_dominance_pct, Decimal::ZERO);
    assert_eq!(inputs.median_24h_return, Decimal::ZERO);
}

#[test]
fn universe_membership_helpers() {
    let universe = sample_universe();

    // enabled() returns only the three enabled eligible entries.
    assert_eq!(universe.enabled().len(), 3);

    // enabled_assets() maps to plain Asset values, same count.
    let assets = universe.enabled_assets();
    assert_eq!(assets.len(), 3);
    assert!(assets.iter().all(|a| a.symbol != "SCAM"));

    // get() finds both enabled and disabled entries by symbol.
    assert_eq!(universe.get("WBNB").map(|a| a.cmc_id), Some(1839));
    assert!(universe.get("SCAM").is_some());
    assert!(universe.get("NOPE").is_none());

    // is_eligible() requires presence AND enabled.
    assert!(universe.is_eligible("CAKE"));
    assert!(!universe.is_eligible("SCAM")); // present but disabled
    assert!(!universe.is_eligible("NOPE")); // absent

    assert_eq!(universe.len(), 4);
    assert!(!universe.is_empty());
}

#[test]
fn liquidity_floor_and_consumption() {
    let state = asset_state("WBNB", 80, vec![], Some(dec!(1_000_000)));

    // Floor: clears at or below liquidity, fails above.
    assert!(liquidity::meets_liquidity_floor(&state, dec!(500_000)));
    assert!(liquidity::meets_liquidity_floor(&state, dec!(1_000_000)));
    assert!(!liquidity::meets_liquidity_floor(&state, dec!(2_000_000)));

    // Missing liquidity never clears any floor.
    let no_liq = asset_state("WBNB", 80, vec![], None);
    assert!(!liquidity::meets_liquidity_floor(&no_liq, dec!(1)));

    // Consumption: a 100k trade against 1M liquidity consumes 10%.
    let consumption =
        liquidity::liquidity_consumption(&state, dec!(100_000)).expect("has liquidity");
    assert_eq!(consumption, dec!(10));

    // No liquidity -> no consumption estimate.
    assert!(liquidity::liquidity_consumption(&no_liq, dec!(100_000)).is_none());
}

#[test]
fn security_passes_clean_asset() {
    let clean = asset_state("WBNB", 80, vec![], Some(dec!(1_000_000)));
    assert!(security::is_tradeable(&clean, 60));
}

#[test]
fn security_blocks_on_honeypot_flag() {
    let flagged = asset_state(
        "RUG",
        90,
        vec!["honeypot".to_string()],
        Some(dec!(1_000_000)),
    );
    // High score is irrelevant once a blocking flag is present.
    assert!(!security::is_tradeable(&flagged, 60));
}

#[test]
fn security_blocks_on_low_safety_score() {
    let low_score = asset_state("WEAK", 30, vec![], Some(dec!(1_000_000)));
    assert!(!security::is_tradeable(&low_score, 60));
}

#[test]
fn snapshot_get_and_global_state_roundtrip() {
    let state = asset_state("WBNB", 80, vec![], Some(dec!(1_000_000)));
    let snapshot = MarketSnapshot {
        timestamp_ms: common::time::now_ms(),
        assets: vec![state],
        fear_greed: Some(FearGreedSnapshot {
            value: 55,
            classification: "Neutral".to_string(),
            updated_ms: 0,
        }),
        global_market: Some(GlobalMarketState {
            total_market_cap_usd: dec!(2_400_000_000_000),
            btc_dominance_pct: dec!(52.30),
        }),
    };

    assert!(snapshot.get("WBNB").is_some());
    assert!(snapshot.get("MISSING").is_none());
    assert!(validator::validate(&snapshot).is_ok());
}
