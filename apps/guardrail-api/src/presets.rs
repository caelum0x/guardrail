//! Strategy preset overrides for the research endpoints.
//!
//! Presets are named bundles of strategy tuning parameters loaded from
//! `configs/strategy_presets.json`. They let the read-only research endpoints
//! show how the same risk policy behaves under different aggressiveness
//! profiles. The risk policy's position cap is always preserved — presets only
//! adjust scoring thresholds, position count, and the stable reserve floor.

use serde::Deserialize;
use std::collections::HashMap;

/// Path to the preset definitions file.
const PRESETS_PATH: &str = "configs/strategy_presets.json";

/// Default preset name applied when none is requested.
pub const DEFAULT_PRESET: &str = "balanced";

/// Optional override fields for a single named preset. All fields are optional
/// so a partial preset definition leaves the remaining config defaults intact.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PresetOverrides {
    pub min_score_to_enter: Option<f64>,
    pub min_score_to_hold: Option<f64>,
    pub max_positions: Option<u32>,
    pub target_stable_reserve_pct: Option<f64>,
}

/// Resolve the requested preset name, returning the matching overrides.
///
/// Falls back silently to an empty (no-op) override set when the preset file is
/// missing, unparseable, or the requested name is not defined.
pub fn resolve(preset: Option<&str>) -> PresetOverrides {
    let name = preset.unwrap_or(DEFAULT_PRESET);
    load_presets()
        .and_then(|mut presets| presets.remove(name))
        .unwrap_or_default()
}

/// The preset name that will be reported back, regardless of whether it exists.
pub fn requested_name(preset: Option<&str>) -> String {
    preset.unwrap_or(DEFAULT_PRESET).to_string()
}

/// Apply preset overrides onto a strategy config, preserving the policy cap.
///
/// `max_position_weight_pct` is deliberately left untouched so the risk
/// policy's hard position cap is never overridden by a preset.
pub fn apply(
    mut cfg: strategy_engine::StrategyConfig,
    overrides: &PresetOverrides,
) -> strategy_engine::StrategyConfig {
    if let Some(value) = overrides.min_score_to_enter {
        cfg.min_score_to_enter = value;
    }
    if let Some(value) = overrides.min_score_to_hold {
        cfg.min_score_to_hold = value;
    }
    if let Some(value) = overrides.max_positions {
        cfg.max_positions = value;
    }
    if let Some(value) = overrides.target_stable_reserve_pct {
        cfg.target_stable_reserve_pct = value;
    }
    cfg
}

fn load_presets() -> Option<HashMap<String, PresetOverrides>> {
    let raw = std::fs::read_to_string(PRESETS_PATH).ok()?;
    serde_json::from_str(&raw).ok()
}
