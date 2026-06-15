//! The tradable universe: the eligible-asset allowlist loaded from disk.

use common::{Asset, EligibleAsset};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Universe {
    assets: Vec<EligibleAsset>,
}

impl Universe {
    pub fn new(assets: Vec<EligibleAsset>) -> Self {
        Universe { assets }
    }

    /// Load the eligible-asset list from a JSON file.
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let assets: Vec<EligibleAsset> = serde_json::from_str(&raw)?;
        Ok(Universe::new(assets))
    }

    /// All enabled eligible assets.
    pub fn enabled(&self) -> Vec<&EligibleAsset> {
        self.assets.iter().filter(|a| a.enabled).collect()
    }

    /// Enabled assets as plain `Asset` values for the data layer.
    pub fn enabled_assets(&self) -> Vec<Asset> {
        self.enabled().into_iter().map(Asset::from).collect()
    }

    /// Look up the eligible entry for a symbol.
    pub fn get(&self, symbol: &str) -> Option<&EligibleAsset> {
        self.assets.iter().find(|a| a.symbol == symbol)
    }

    /// Is a symbol present and enabled?
    pub fn is_eligible(&self, symbol: &str) -> bool {
        self.assets.iter().any(|a| a.symbol == symbol && a.enabled)
    }

    pub fn len(&self) -> usize {
        self.assets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}
