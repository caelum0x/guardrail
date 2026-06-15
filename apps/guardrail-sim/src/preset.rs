//! Strategy preset loading and application.

use std::collections::HashMap;

use strategy_engine::StrategyConfig;

/// Path to the selectable strategy presets file.
pub const STRATEGY_PRESETS_PATH: &str = "configs/strategy_presets.json";

/// Default preset applied when `--preset` is not supplied.
pub const DEFAULT_PRESET: &str = "balanced";

/// Optional, leniently-parsed overrides for a named strategy preset. Each field
/// is applied to the base `StrategyConfig` only when present in the JSON.
#[derive(Debug, Default, Clone, serde::Deserialize)]
pub struct PresetOverrides {
    pub min_score_to_enter: Option<f64>,
    pub min_score_to_hold: Option<f64>,
    pub max_positions: Option<u32>,
    pub rebalance_threshold_pct: Option<f64>,
    pub target_stable_reserve_pct: Option<f64>,
}

impl PresetOverrides {
    /// Apply present fields onto `cfg`, leaving the rest (including the
    /// policy-derived `max_position_weight_pct`) untouched.
    pub fn apply(&self, mut cfg: StrategyConfig) -> StrategyConfig {
        if let Some(v) = self.min_score_to_enter {
            cfg.min_score_to_enter = v;
        }
        if let Some(v) = self.min_score_to_hold {
            cfg.min_score_to_hold = v;
        }
        if let Some(v) = self.max_positions {
            cfg.max_positions = v;
        }
        if let Some(v) = self.rebalance_threshold_pct {
            cfg.rebalance_threshold_pct = v;
        }
        if let Some(v) = self.target_stable_reserve_pct {
            cfg.target_stable_reserve_pct = v;
        }
        cfg
    }
}

/// Load the full preset map. Returns an error string (not a panic) on a missing
/// or malformed file, so callers can fall back to default behavior.
pub fn load_presets() -> Result<HashMap<String, PresetOverrides>, String> {
    let raw = std::fs::read_to_string(STRATEGY_PRESETS_PATH)
        .map_err(|_| format!("preset file '{STRATEGY_PRESETS_PATH}' not found"))?;
    serde_json::from_str(&raw).map_err(|e| format!("failed to parse '{STRATEGY_PRESETS_PATH}' ({e})"))
}

/// The base strategy config: production entry/hold scores with a position cap
/// derived from the risk policy.
pub fn base_config(cap: f64) -> StrategyConfig {
    StrategyConfig {
        max_position_weight_pct: cap,
        min_score_to_enter: 0.55,
        min_score_to_hold: 0.45,
        ..StrategyConfig::default()
    }
}

/// Build the config for a named preset, plus a human-readable note about which
/// preset is active. Falls back to the base config on any load/lookup miss.
pub fn strategy_config(cap: f64, preset: &str) -> (StrategyConfig, String) {
    let base = base_config(cap);
    match load_presets() {
        Ok(presets) => match presets.get(preset) {
            Some(o) => (o.apply(base), format!("active preset: {preset}")),
            None => (
                base,
                format!("note: preset '{preset}' not found in '{STRATEGY_PRESETS_PATH}'; using default config"),
            ),
        },
        Err(e) => (base, format!("note: {e}; using default config")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_apply_only_present_fields() {
        let base = base_config(10.0);
        let o = PresetOverrides {
            min_score_to_enter: Some(0.7),
            ..Default::default()
        };
        let cfg = o.apply(base.clone());
        assert_eq!(cfg.min_score_to_enter, 0.7);
        // Untouched fields keep the base value.
        assert_eq!(cfg.min_score_to_hold, base.min_score_to_hold);
        assert_eq!(cfg.max_position_weight_pct, 10.0);
    }
}
