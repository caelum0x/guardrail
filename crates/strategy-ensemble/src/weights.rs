//! The ensemble weight table: typed config plus loading/parsing.
//!
//! The canonical source is `skills/ensemble.json` (per-regime, per-skill blend
//! weights). The content is embedded at compile time via [`EMBEDDED_CONFIG`] so
//! the library is self-contained and offline-safe, but callers can also parse a
//! config from an arbitrary path or string at runtime.

use crate::error::EnsembleError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use strategy_engine::MarketRegime;

/// The default reserve / quote leg symbol when the config omits one.
pub const DEFAULT_RESERVE_SYMBOL: &str = "USDT";
/// The default maximum risk allocation (rest is held as reserve).
pub const DEFAULT_MAX_RISK_ALLOCATION_PCT: f64 = 100.0;

/// The `skills/ensemble.json` content, embedded at compile time.
///
/// Using [`include_str!`] keeps the crate self-contained: [`EnsembleConfig::embedded`]
/// never touches the filesystem and so works in any sandbox.
pub const EMBEDDED_CONFIG: &str = include_str!("../../../skills/ensemble.json");

/// One regime's blend configuration: a rationale plus per-skill weights.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegimeWeights {
    /// Human-readable explanation of why these weights were chosen.
    #[serde(default)]
    pub rationale: String,
    /// Per-skill blend weights for this regime (skill name -> weight).
    ///
    /// Stored in a [`BTreeMap`] so iteration order is deterministic.
    #[serde(default)]
    pub weights: BTreeMap<String, f64>,
}

impl RegimeWeights {
    /// Coerce and renormalize the raw blend weights so they sum to `1.0`.
    ///
    /// Non-positive or non-finite entries are dropped. When the surviving
    /// weights sum to a positive value they are renormalized to `1.0` (robust
    /// to hand-edited configs that drift off 1.0); an all-zero/empty set yields
    /// an empty map.
    pub fn normalized(&self) -> BTreeMap<String, f64> {
        let clean: BTreeMap<String, f64> = self
            .weights
            .iter()
            .filter(|(_, &w)| w.is_finite() && w > 0.0)
            .map(|(s, &w)| (s.clone(), w))
            .collect();
        let total: f64 = clean.values().sum();
        if total <= 0.0 {
            return BTreeMap::new();
        }
        clean
            .into_iter()
            .map(|(skill, weight)| (skill, weight / total))
            .collect()
    }
}

/// The fully-typed ensemble weight table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnsembleConfig {
    /// Config schema version (informational).
    #[serde(default)]
    pub version: String,
    /// The symbol holding the unallocated remainder.
    #[serde(default = "default_reserve_symbol")]
    pub reserve_symbol: String,
    /// Risk allocation can never exceed this; the rest becomes reserve.
    #[serde(default = "default_max_risk")]
    pub max_risk_allocation_pct: f64,
    /// Per-regime blend configuration, keyed by the regime's `snake_case` name.
    #[serde(default)]
    pub regimes: BTreeMap<String, RegimeWeights>,
}

fn default_reserve_symbol() -> String {
    DEFAULT_RESERVE_SYMBOL.to_string()
}

fn default_max_risk() -> f64 {
    DEFAULT_MAX_RISK_ALLOCATION_PCT
}

impl EnsembleConfig {
    /// Parse the config embedded at compile time from `skills/ensemble.json`.
    ///
    /// This is the recommended entry point: it never touches the filesystem.
    pub fn embedded() -> Result<Self, EnsembleError> {
        Self::from_json(EMBEDDED_CONFIG)
    }

    /// Parse a config from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, EnsembleError> {
        let cfg: EnsembleConfig =
            serde_json::from_str(json).map_err(EnsembleError::Parse)?;
        cfg.validated()
    }

    /// Parse a config from a JSON file at `path`.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, EnsembleError> {
        let path = path.as_ref();
        let raw = std::fs::read_to_string(path).map_err(|source| EnsembleError::Read {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_json(&raw)
    }

    /// Normalize defaults that may be missing or out of range.
    fn validated(mut self) -> Result<Self, EnsembleError> {
        if self.reserve_symbol.trim().is_empty() {
            self.reserve_symbol = DEFAULT_RESERVE_SYMBOL.to_string();
        }
        if !self.max_risk_allocation_pct.is_finite() || self.max_risk_allocation_pct <= 0.0 {
            self.max_risk_allocation_pct = DEFAULT_MAX_RISK_ALLOCATION_PCT;
        }
        Ok(self)
    }

    /// Look up the blend config for a [`MarketRegime`] using its `snake_case` key.
    pub fn regime(&self, regime: MarketRegime) -> Option<&RegimeWeights> {
        self.regimes.get(regime.as_str())
    }
}
