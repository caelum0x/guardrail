//! Integration tests for the deterministic CMC mock client.

use cmc_client::{CmcDataSource, MockCmcClient};
use common::{Asset, AssetCategory, Decimal};

fn asset(symbol: &str, cmc_id: u64, category: AssetCategory) -> Asset {
    Asset {
        symbol: symbol.to_string(),
        cmc_id,
        chain_id: 56,
        contract_address: "0x0000000000000000000000000000000000000000".to_string(),
        decimals: 18,
        category,
    }
}

#[tokio::test]
async fn latest_quotes_prices_are_deterministic_per_call() {
    let client = MockCmcClient::new();
    let assets = vec![
        asset("CAKE", 7186, AssetCategory::DeFi),
        asset("BNB", 1839, AssetCategory::Core),
    ];

    let first = client.latest_quotes(&assets).await.unwrap();
    let second = client.latest_quotes(&assets).await.unwrap();

    // Prices are symbol-derived and stable across calls even though the
    // internal tick advances.
    for (a, b) in first.iter().zip(second.iter()) {
        assert_eq!(a.symbol, b.symbol);
        assert_eq!(a.price_usd, b.price_usd, "price for {} drifted", a.symbol);
    }
}

#[tokio::test]
async fn latest_quotes_stable_prices_are_about_one() {
    let client = MockCmcClient::new();
    let assets = vec![asset("USDT", 825, AssetCategory::Stable)];
    let quotes = client.latest_quotes(&assets).await.unwrap();

    let usdt = quotes.iter().find(|q| q.symbol == "USDT").unwrap();
    assert_eq!(usdt.price_usd, Decimal::ONE);
    // Stables have flat momentum in the mock tape.
    assert_eq!(usdt.percent_change_1h, Some(Decimal::ZERO));
    assert_eq!(usdt.percent_change_24h, Some(Decimal::ZERO));
}

#[tokio::test]
async fn latest_quotes_non_stable_prices_above_one() {
    let client = MockCmcClient::new();
    let assets = vec![asset("CAKE", 7186, AssetCategory::DeFi)];
    let quotes = client.latest_quotes(&assets).await.unwrap();
    assert!(quotes[0].price_usd > Decimal::ONE);
}
