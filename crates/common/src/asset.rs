use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// High-level classification used by the strategy and risk layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetCategory {
    Stable,
    Core,
    DeFi,
    Meme,
    Ai,
    Rwa,
    Infrastructure,
    Other,
}

impl AssetCategory {
    pub fn is_stable(&self) -> bool {
        matches!(self, AssetCategory::Stable)
    }
}

/// A tradable asset on a specific chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub symbol: String,
    pub cmc_id: u64,
    pub chain_id: u64,
    pub contract_address: String,
    pub decimals: u8,
    pub category: AssetCategory,
}

fn default_true() -> bool {
    true
}

/// An entry in the eligible-asset allowlist (`configs/eligible_assets.bsc.json`).
///
/// Trades against assets that are missing or `enabled = false` here never
/// reach the executor — the risk engine rejects them first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EligibleAsset {
    pub symbol: String,
    pub cmc_id: u64,
    pub chain_id: u64,
    pub contract_address: String,
    pub decimals: u8,
    pub category: AssetCategory,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub min_liquidity_usd: Decimal,
    #[serde(default)]
    pub min_volume_24h_usd: Decimal,
}

impl From<&EligibleAsset> for Asset {
    fn from(e: &EligibleAsset) -> Self {
        Asset {
            symbol: e.symbol.clone(),
            cmc_id: e.cmc_id,
            chain_id: e.chain_id,
            contract_address: e.contract_address.clone(),
            decimals: e.decimals,
            category: e.category,
        }
    }
}
