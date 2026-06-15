//! Run-report (`data/run_report.json`) parsing helpers.

use serde_json::Value;

/// Load and parse the run report JSON, if present.
pub fn load_report(path: &str) -> Option<Value> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Parse a string-or-number JSON field into f64.
pub fn num(report: &Value, key: &str) -> Option<f64> {
    match report.get(key) {
        Some(Value::String(s)) => s.parse().ok(),
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

/// Current UTC time in milliseconds (for report-age computation).
pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn num_parses_string_and_number() {
        let r = json!({ "a": "12.5", "b": 7, "c": true });
        assert_eq!(num(&r, "a"), Some(12.5));
        assert_eq!(num(&r, "b"), Some(7.0));
        assert_eq!(num(&r, "c"), None);
        assert_eq!(num(&r, "missing"), None);
    }
}
