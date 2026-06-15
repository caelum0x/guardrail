//! Tests for asset domain types: AssetCategory serde, EligibleAsset -> Asset,
//! and serde defaults on EligibleAsset.

use common::asset::{Asset, AssetCategory, EligibleAsset};
use common::Decimal;

#[test]
fn asset_category_serde_round_trip_lowercase() {
    let cases = [
        (AssetCategory::Stable, "\"stable\""),
        (AssetCategory::DeFi, "\"defi\""),
        (AssetCategory::Core, "\"core\""),
        (AssetCategory::Meme, "\"meme\""),
        (AssetCategory::Ai, "\"ai\""),
        (AssetCategory::Rwa, "\"rwa\""),
        (AssetCategory::Infrastructure, "\"infrastructure\""),
        (AssetCategory::Other, "\"other\""),
    ];

    for (cat, expected_json) in cases {
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, expected_json, "serialization mismatch for {cat:?}");

        let back: AssetCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cat, "round-trip mismatch for {cat:?}");
    }
}

#[test]
fn asset_category_is_stable() {
    assert!(AssetCategory::Stable.is_stable());
    assert!(!AssetCategory::Core.is_stable());
}

fn sample_eligible() -> EligibleAsset {
    EligibleAsset {
        symbol: "CAKE".to_string(),
        cmc_id: 7186,
        chain_id: 56,
        contract_address: "0x0e09fabb73bd3ade0a17ecc321fd13a19e81ce82".to_string(),
        decimals: 18,
        category: AssetCategory::DeFi,
        enabled: true,
        min_liquidity_usd: Decimal::from(1000),
        min_volume_24h_usd: Decimal::from(500),
    }
}

#[test]
fn eligible_asset_into_asset_copies_fields() {
    let eligible = sample_eligible();
    let asset: Asset = (&eligible).into();

    assert_eq!(asset.symbol, eligible.symbol);
    assert_eq!(asset.cmc_id, eligible.cmc_id);
    assert_eq!(asset.chain_id, eligible.chain_id);
    assert_eq!(asset.contract_address, eligible.contract_address);
    assert_eq!(asset.decimals, eligible.decimals);
    assert_eq!(asset.category, eligible.category);
}

#[test]
fn eligible_asset_deserializes_with_defaults() {
    // No `enabled`, `min_liquidity_usd`, or `min_volume_24h_usd` provided.
    let json = r#"{
        "symbol": "USDT",
        "cmc_id": 825,
        "chain_id": 56,
        "contract_address": "0x55d398326f99059ff775485246999027b3197955",
        "decimals": 18,
        "category": "stable"
    }"#;

    let eligible: EligibleAsset = serde_json::from_str(json).unwrap();
    assert_eq!(eligible.symbol, "USDT");
    assert_eq!(eligible.category, AssetCategory::Stable);
    // enabled defaults to true.
    assert!(eligible.enabled);
    assert_eq!(eligible.min_liquidity_usd, Decimal::ZERO);
    assert_eq!(eligible.min_volume_24h_usd, Decimal::ZERO);
}

#[test]
fn eligible_asset_full_serde_round_trip() {
    let eligible = sample_eligible();
    let json = serde_json::to_string(&eligible).unwrap();
    let back: EligibleAsset = serde_json::from_str(&json).unwrap();
    assert_eq!(back, eligible);
}
