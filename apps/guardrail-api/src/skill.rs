//! CMC Skill artifact endpoint.
//!
//! Surfaces the Track 2 CMC Skill (`cmc-regime-routed-alpha`) by reading its
//! `skill.yaml` and `README.md` as raw text and listing the example filenames.
//! The API has no YAML parser, so the raw text is returned verbatim for the
//! dashboard to render. Read-only and side-effect free; missing files degrade
//! gracefully to empty strings / empty lists.

use axum::Json;
use serde_json::{json, Value};

const SKILL_DIR: &str = "skills/cmc-regime-routed-alpha";

/// Reads a file to a string, returning an empty string when it is missing or
/// unreadable so the endpoint never fails on absent artifacts.
fn read_text_or_empty(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Lists the example filenames in the skill's `examples/` directory, sorted for
/// deterministic output. Returns an empty list when the directory is missing.
fn example_filenames(dir: &str) -> Vec<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut names: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

pub async fn skill() -> Json<Value> {
    let skill_yaml = read_text_or_empty(&format!("{SKILL_DIR}/skill.yaml"));
    let readme = read_text_or_empty(&format!("{SKILL_DIR}/README.md"));
    let examples = example_filenames(&format!("{SKILL_DIR}/examples"));

    Json(json!({
        "name": "cmc-regime-routed-alpha",
        "skill_yaml": skill_yaml,
        "readme": readme,
        "examples": examples,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_empty_string() {
        assert_eq!(read_text_or_empty("does/not/exist.yaml"), "");
    }

    #[test]
    fn missing_dir_yields_empty_list() {
        assert!(example_filenames("does/not/exist").is_empty());
    }

    #[tokio::test]
    async fn skill_returns_expected_shape() {
        let Json(value) = skill().await;
        assert_eq!(value["name"], "cmc-regime-routed-alpha");
        assert!(value["skill_yaml"].is_string());
        assert!(value["readme"].is_string());
        assert!(value["examples"].is_array());
    }
}
