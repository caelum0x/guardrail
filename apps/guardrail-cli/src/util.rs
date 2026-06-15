//! Pure, reusable helpers shared across the CLI's command runners.
//!
//! These were extracted verbatim from `main.rs` to keep that file focused on the
//! command dispatch surface. Nothing here performs command-specific logic — only
//! JSON/decimal coercion, filesystem counting, and small numeric formulas.

use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Parse a decimal from a string, returning `None` on any parse error.
pub fn decimal_from_str(value: &str) -> Option<Decimal> {
    Decimal::from_str(value).ok()
}

/// Recursively count every file under `path` (0 if it is not a readable dir).
pub fn count_files_cli(path: &std::path::Path) -> usize {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                count_files_cli(&path)
            } else {
                1
            }
        })
        .sum()
}

/// Recursively count files with the given extension under `path`.
pub fn count_ext_cli(path: &std::path::Path, extension: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                count_ext_cli(&path, extension)
            } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
                1
            } else {
                0
            }
        })
        .sum()
}

/// Coerce an optional JSON value into a [`Decimal`] across number/string shapes.
pub fn decimal_value(value: Option<&serde_json::Value>) -> Option<Decimal> {
    value
        .and_then(serde_json::Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| value.and_then(serde_json::Value::as_i64).map(Decimal::from))
        .or_else(|| value.and_then(serde_json::Value::as_u64).map(Decimal::from))
        .or_else(|| {
            value
                .and_then(serde_json::Value::as_str)
                .and_then(decimal_from_str)
        })
}

/// Gas cost in USD: `gas_units * gas_price_gwei / 1e9 * native_price`.
pub fn gas_cost_usd(gas_units: Decimal, gas_price_gwei: Decimal, native_price: Decimal) -> Decimal {
    gas_units * gas_price_gwei / Decimal::from(1_000_000_000u64) * native_price
}

/// Cost expressed in basis points of notional (0 when notional is non-positive).
pub fn cost_bps(cost: Decimal, notional: Decimal) -> Decimal {
    if notional <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        cost / notional * Decimal::from(10_000)
    }
}

/// Read a keyed [`Decimal`] field from a JSON object, falling back to `default`.
pub fn json_decimal_or(value: &serde_json::Value, key: &str, default: Decimal) -> Decimal {
    value
        .get(key)
        .and_then(serde_json::Value::as_f64)
        .and_then(Decimal::from_f64)
        .or_else(|| {
            value
                .get(key)
                .and_then(serde_json::Value::as_i64)
                .map(Decimal::from)
        })
        .or_else(|| {
            value
                .get(key)
                .and_then(serde_json::Value::as_u64)
                .map(Decimal::from)
        })
        .or_else(|| {
            value
                .get(key)
                .and_then(serde_json::Value::as_str)
                .and_then(decimal_from_str)
        })
        .unwrap_or(default)
}

/// Parse a `{category: shock}` JSON object into a sorted map of decimals.
pub fn scenario_shock_map(value: &serde_json::Value) -> BTreeMap<String, Decimal> {
    let mut shocks = BTreeMap::new();
    if let Some(object) = value.as_object() {
        for (category, shock) in object {
            let parsed = shock
                .as_f64()
                .and_then(Decimal::from_f64)
                .or_else(|| shock.as_str().and_then(decimal_from_str))
                .unwrap_or(Decimal::ZERO);
            shocks.insert(category.clone(), parsed);
        }
    }
    shocks
}

/// Read a keyed field that the agent persisted as a numeric string into a
/// [`Decimal`], defaulting to zero.
pub fn json_decimal_field(value: &serde_json::Value, field: &str) -> Decimal {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .and_then(decimal_from_str)
        .unwrap_or(Decimal::ZERO)
}

/// Current Unix time in milliseconds (0 if the clock is before the epoch).
pub fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Read a string field, or `"n/a"` when absent.
pub fn json_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}

/// Read a numeric field that may be a JSON number or a numeric string (the agent
/// persists decimals as strings to preserve precision).
pub fn json_f64(value: &serde_json::Value, key: &str) -> Option<f64> {
    let field = value.get(key)?;
    field
        .as_f64()
        .or_else(|| field.as_str().and_then(|s| s.parse::<f64>().ok()))
}

/// Format an optional numeric field as a fixed-precision string, or `"n/a"`.
pub fn json_num_fmt(value: &serde_json::Value, key: &str) -> String {
    json_f64(value, key)
        .map(|n| format!("{n:.2}"))
        .unwrap_or_else(|| "n/a".to_string())
}

/// Read a nested `metrics.<key>` numeric field.
pub fn metric_f64(value: &serde_json::Value, key: &str) -> Option<f64> {
    value.get("metrics").and_then(|m| json_f64(m, key))
}

/// Format a nested `metrics.<key>` numeric field, or `"n/a"`.
pub fn metric_fmt(value: &serde_json::Value, key: &str) -> String {
    metric_f64(value, key)
        .map(|n| format!("{n:.2}"))
        .unwrap_or_else(|| "n/a".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_parsing_and_bps() {
        assert_eq!(decimal_from_str("1.5"), Some(Decimal::new(15, 1)));
        assert_eq!(decimal_from_str("nope"), None);
        assert_eq!(cost_bps(Decimal::ONE, Decimal::from(10_000)), Decimal::ONE);
        assert_eq!(cost_bps(Decimal::ONE, Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn json_decimal_or_falls_back() {
        let v = serde_json::json!({ "a": "2.5", "b": 3 });
        assert_eq!(json_decimal_or(&v, "a", Decimal::ZERO), Decimal::new(25, 1));
        assert_eq!(json_decimal_or(&v, "b", Decimal::ZERO), Decimal::from(3));
        assert_eq!(
            json_decimal_or(&v, "missing", Decimal::from(9)),
            Decimal::from(9)
        );
    }
}
