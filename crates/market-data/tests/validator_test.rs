//! Integration tests for snapshot validation.

use common::constants::MAX_SNAPSHOT_AGE_MS;
use common::time::now_ms;
use common::{Asset, AssetCategory, Decimal};
use market_data::validator::{validate, SnapshotValidity};
use market_data::{AssetMarketState, MarketSnapshot};

fn asset_state(symbol: &str, price: Decimal) -> AssetMarketState {
    AssetMarketState {
        asset: Asset {
            symbol: symbol.to_string(),
            cmc_id: 1,
            chain_id: 56,
            contract_address: "0x0000000000000000000000000000000000000000".to_string(),
            decimals: 18,
            category: AssetCategory::Core,
        },
        price_usd: price,
        volume_24h_usd: Decimal::from(1_000_000),
        market_cap_usd: Some(Decimal::from(10_000_000)),
        liquidity_usd: Some(Decimal::from(1_000_000)),
        ret_1h: Some(Decimal::ZERO),
        ret_24h: Some(Decimal::ZERO),
        volatility_1h: Some(Decimal::ONE),
        safety_score: 80,
        security_flags: vec![],
    }
}

fn snapshot(timestamp_ms: i64, assets: Vec<AssetMarketState>) -> MarketSnapshot {
    MarketSnapshot {
        timestamp_ms,
        assets,
        fear_greed: None,
        global_market: None,
    }
}

#[test]
fn validate_ok_for_fresh_non_empty_priced_snapshot() {
    let snap = snapshot(now_ms(), vec![asset_state("BTC", Decimal::from(50_000))]);
    assert_eq!(validate(&snap), SnapshotValidity::Ok);
    assert!(validate(&snap).is_ok());
}

#[test]
fn validate_empty_when_no_assets() {
    let snap = snapshot(now_ms(), vec![]);
    assert_eq!(validate(&snap), SnapshotValidity::Empty);
    assert!(!validate(&snap).is_ok());
}

#[test]
fn validate_no_prices_when_all_prices_zero() {
    let snap = snapshot(now_ms(), vec![asset_state("BTC", Decimal::ZERO)]);
    assert_eq!(validate(&snap), SnapshotValidity::NoPrices);
}

#[test]
fn validate_stale_when_older_than_max_age() {
    // Timestamp well beyond the staleness window.
    let stale_ts = now_ms() - (MAX_SNAPSHOT_AGE_MS + 60_000);
    let snap = snapshot(stale_ts, vec![asset_state("BTC", Decimal::from(50_000))]);
    match validate(&snap) {
        SnapshotValidity::Stale { age_ms } => {
            assert!(age_ms > MAX_SNAPSHOT_AGE_MS, "age {age_ms} not stale");
        }
        other => panic!("expected Stale, got {other:?}"),
    }
}
