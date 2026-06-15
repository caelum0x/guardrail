//! Track-2 skills / ensemble / preset config checks.

use serde_json::Value;

use crate::check::CheckResult;

/// `skills/INDEX.json` parses and every entry's `spec_file` exists on disk.
pub fn check_skills_index(path: &str) -> CheckResult {
    let name = format!("skills index: {path}");
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::warn(name, format!("absent or unreadable: {err}")),
    };
    let entries: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => return CheckResult::fail(name, format!("invalid JSON: {err}")),
    };
    let Some(arr) = entries.as_array() else {
        return CheckResult::fail(name, "expected a JSON array of skills");
    };
    let missing: Vec<String> = arr
        .iter()
        .filter_map(|e| e.get("spec_file").and_then(Value::as_str))
        .filter(|spec| !std::path::Path::new(spec).exists())
        .map(str::to_string)
        .collect();
    if arr.is_empty() {
        CheckResult::warn(name, "no skills listed")
    } else if missing.is_empty() {
        CheckResult::pass(name, format!("{} skill(s), all spec files present", arr.len()))
    } else {
        CheckResult::fail(name, format!("missing spec file(s): {}", missing.join(", ")))
    }
}

/// `skills/ensemble.json` parses and every regime's weights sum to ~1.0.
pub fn check_ensemble_weights(path: &str) -> CheckResult {
    let name = format!("ensemble weights: {path}");
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::warn(name, format!("absent or unreadable: {err}")),
    };
    let doc: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(err) => return CheckResult::fail(name, format!("invalid JSON: {err}")),
    };
    let Some(regimes) = doc.get("regimes").and_then(Value::as_object) else {
        return CheckResult::fail(name, "missing 'regimes' object");
    };

    let mut off: Vec<String> = Vec::new();
    for (regime, body) in regimes {
        if let Some(weights) = body.get("weights").and_then(Value::as_object) {
            let sum = sum_weights(weights);
            if !weights_ok(sum) {
                off.push(format!("{regime}={sum:.3}"));
            }
        }
    }
    if off.is_empty() {
        CheckResult::pass(name, format!("{} regime(s), all weights sum to ~1.0", regimes.len()))
    } else {
        CheckResult::fail(name, format!("weights not normalized: {}", off.join(", ")))
    }
}

/// `configs/strategy_presets.json` parses as a non-empty object of presets.
pub fn check_strategy_presets(path: &str) -> CheckResult {
    let name = format!("strategy presets: {path}");
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) => return CheckResult::warn(name, format!("absent or unreadable: {err}")),
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(v) => match v.as_object() {
            Some(map) if !map.is_empty() => {
                let names: Vec<&str> = map.keys().map(String::as_str).collect();
                CheckResult::pass(name, format!("presets: {}", names.join(", ")))
            }
            Some(_) => CheckResult::warn(name, "no presets defined"),
            None => CheckResult::fail(name, "expected a JSON object of presets"),
        },
        Err(err) => CheckResult::fail(name, format!("invalid JSON: {err}")),
    }
}

/// Sum the numeric values of a weights map.
fn sum_weights(weights: &serde_json::Map<String, Value>) -> f64 {
    weights.values().filter_map(Value::as_f64).sum()
}

/// True when a weights sum is within tolerance of 1.0.
fn weights_ok(sum: f64) -> bool {
    (sum - 1.0).abs() <= 0.01
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sums_and_tolerance() {
        let w = json!({ "a": 0.35, "b": 0.35, "c": 0.30 });
        let sum = sum_weights(w.as_object().unwrap());
        assert!(weights_ok(sum));
        assert!(!weights_ok(0.8));
        assert!(weights_ok(1.005));
    }
}
