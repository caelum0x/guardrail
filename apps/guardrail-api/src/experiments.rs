//! Backtest experiment tracking endpoint.
//!
//! Surfaces the experiment files written by the CLI to `data/experiments/<tag>.json`.
//! Reads every `*.json` file in that directory, parses each independently, sorts by
//! `created_ms` ascending, and returns the parsed objects. Read-only and side-effect
//! free. Missing directory, non-JSON files, and unreadable or malformed files are
//! skipped gracefully rather than failing the request.

use axum::Json;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

const EXPERIMENTS_DIR: &str = "data/experiments";

pub async fn experiments() -> Json<Value> {
    let mut parsed: Vec<Value> = Vec::new();

    let entries = match fs::read_dir(Path::new(EXPERIMENTS_DIR)) {
        Ok(entries) => entries,
        Err(_) => return Json(json!({ "count": 0, "experiments": [] })),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let is_json = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("json"))
            .unwrap_or(false);
        if !is_json {
            continue;
        }

        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(_) => continue,
        };

        match serde_json::from_str::<Value>(&contents) {
            Ok(value) => parsed.push(value),
            Err(_) => continue,
        }
    }

    parsed.sort_by(|a, b| {
        let a_ms = a.get("created_ms").and_then(Value::as_i64).unwrap_or(0);
        let b_ms = b.get("created_ms").and_then(Value::as_i64).unwrap_or(0);
        a_ms.cmp(&b_ms)
    });

    Json(json!({ "count": parsed.len(), "experiments": parsed }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_empty_when_dir_missing_or_present() {
        let Json(value) = experiments().await;
        assert!(value.get("count").is_some());
        assert!(value["experiments"].is_array());
        let count = value["count"].as_u64().expect("count should be a number");
        let len = value["experiments"]
            .as_array()
            .expect("experiments should be an array")
            .len() as u64;
        assert_eq!(count, len);
    }
}
