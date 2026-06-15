//! Run report model loaded from the JSON report file.

use serde_json::Value;

use crate::field_str;

/// A single open position from the run report.
#[derive(Debug, Clone)]
pub struct Position {
    pub symbol: String,
    pub value_usd: String,
    pub weight_pct: String,
}

/// Parsed view of the run report. All fields are strings (or placeholders) so
/// rendering never has to deal with missing or mistyped JSON.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub available: bool,
    pub run_id: String,
    pub mode: String,
    pub regime: String,
    pub nav_usd: String,
    pub total_drawdown_pct: String,
    pub kill_switch: String,
    pub positions: Vec<Position>,
}

impl RunReport {
    /// A placeholder report used when the file is missing or unreadable.
    pub fn unavailable() -> Self {
        Self {
            available: false,
            run_id: "—".to_string(),
            mode: "—".to_string(),
            regime: "—".to_string(),
            nav_usd: "—".to_string(),
            total_drawdown_pct: "—".to_string(),
            kill_switch: "—".to_string(),
            positions: Vec::new(),
        }
    }

    /// Loads and parses the run report from `path`, returning a placeholder on
    /// any error so the cockpit never panics.
    pub fn load(path: &str) -> Self {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(_) => return Self::unavailable(),
        };
        let value: Value = match serde_json::from_str(&contents) {
            Ok(value) => value,
            Err(_) => return Self::unavailable(),
        };
        Self::from_value(&value)
    }

    fn from_value(value: &Value) -> Self {
        let positions = value
            .get("positions")
            .and_then(Value::as_array)
            .map(|items| items.iter().map(Position::from_value).collect())
            .unwrap_or_default();

        Self {
            available: true,
            run_id: field_str(value, "run_id"),
            mode: field_str(value, "mode"),
            regime: field_str(value, "regime"),
            nav_usd: field_str(value, "nav_usd"),
            total_drawdown_pct: field_str(value, "total_drawdown_pct"),
            kill_switch: field_str(value, "kill_switch"),
            positions,
        }
    }
}

impl Position {
    fn from_value(value: &Value) -> Self {
        Self {
            symbol: field_str(value, "symbol"),
            value_usd: field_str(value, "value_usd"),
            weight_pct: field_str(value, "weight_pct"),
        }
    }
}
