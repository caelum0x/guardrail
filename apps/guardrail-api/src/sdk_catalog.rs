//! Product-owned BNB Agent SDK catalog endpoint.
//!
//! Introspects the in-repo SDK integration under `integrations/bnbagent-sdk`
//! and reports module/example/test inventory for the operator dashboard.

use axum::Json;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const ROOT: &str = "integrations/bnbagent-sdk";

pub async fn sdk_catalog() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let root = Path::new(ROOT);
    let module_names = [
        "erc8004", "erc8183", "x402", "signing", "wallets", "storage", "erc20", "core", "networks",
    ];
    let modules = module_names
        .iter()
        .map(|name| {
            let path = root.join("bnbagent").join(name);
            json!({
                "name": name,
                "path": path.to_string_lossy(),
                "present": path.exists(),
                "files": count_files(&path)
            })
        })
        .collect::<Vec<_>>();
    let examples = child_dirs(&root.join("examples"));
    let top_files = ["README.md", "ARCHITECTURE.md", "pyproject.toml", "LICENSE"]
        .iter()
        .map(|name| {
            let path = root.join(name);
            json!({
                "name": name,
                "path": path.to_string_lossy(),
                "present": path.exists(),
                "bytes": std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "root": ROOT,
        "status": if root.exists() { "present" } else { "missing" },
        "summary": {
            "files": count_files(root),
            "modules": modules.len(),
            "modules_present": modules.iter().filter(|m| m.get("present").and_then(Value::as_bool).unwrap_or(false)).count(),
            "examples": examples.len(),
            "tests": count_files(&root.join("tests")),
            "abis": count_matching(root, "json")
        },
        "modules": modules,
        "examples": examples,
        "top_files": top_files
    }))
}

fn child_dirs(path: &Path) -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut rows = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let path = entry.path();
            json!({
                "name": entry.file_name().to_string_lossy(),
                "path": path.to_string_lossy(),
                "files": count_files(&path)
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        a.get("name")
            .and_then(Value::as_str)
            .cmp(&b.get("name").and_then(Value::as_str))
    });
    rows
}

fn count_files(path: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path = entry.path();
            if path.is_dir() {
                count_files(&path)
            } else {
                1
            }
        })
        .sum()
}

fn count_matching(path: &Path, extension: &str) -> usize {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| {
            let path: PathBuf = entry.path();
            if path.is_dir() {
                count_matching(&path, extension)
            } else if path.extension().and_then(|ext| ext.to_str()) == Some(extension) {
                1
            } else {
                0
            }
        })
        .sum()
}
