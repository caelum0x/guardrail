//! Track-2 Skill catalog endpoint.
//!
//! Reads `skills/INDEX.json` (the registry of published Track-2 Skills) and
//! returns it as a typed projection together with a small summary. Read-only
//! and side-effect free; a missing or unparseable index degrades to an empty
//! catalog rather than an error so the endpoint never panics.

use axum::Json;
use serde_json::{json, Value};

const INDEX_PATH: &str = "skills/INDEX.json";

pub async fn skills() -> Json<Value> {
    let entries = load_index();
    let count = entries.len();
    let ids: Vec<&str> = entries
        .iter()
        .filter_map(|entry| entry.get("id").and_then(Value::as_str))
        .collect();

    Json(json!({
        "index_path": INDEX_PATH,
        "count": count,
        "ids": ids,
        "skills": entries,
    }))
}

/// Loads and parses the Skill index as an array of objects. Returns an empty
/// vector when the file is missing, unreadable, or not a JSON array.
fn load_index() -> Vec<Value> {
    std::fs::read_to_string(INDEX_PATH)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
}
