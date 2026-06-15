//! Submission audit manifest endpoint.
//!
//! Builds a read-only inventory of configured evidence artifacts and operator
//! routes. File entries include presence, size, and stable SHA-256 hash.

use axum::Json;
use serde_json::{json, Value};

const MANIFEST: &str = "configs/audit/export_manifest.json";

pub async fn audit_manifest() -> Json<Value> {
    match build() {
        Ok(value) => Json(value),
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

fn build() -> anyhow::Result<Value> {
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(MANIFEST)?)?;
    let artifacts = manifest
        .get("artifacts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut present = 0usize;
    let mut missing_required = 0usize;
    let mut total_bytes = 0u64;
    let mut rows = Vec::new();

    for artifact in artifacts {
        let label = artifact
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("artifact");
        let path = artifact.get("path").and_then(Value::as_str).unwrap_or("");
        let required = artifact
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let bytes = std::fs::read(path).ok();
        let exists = bytes.is_some();
        if exists {
            present += 1;
        } else if required {
            missing_required += 1;
        }
        let size_bytes = bytes.as_ref().map(|b| b.len() as u64).unwrap_or(0);
        total_bytes += size_bytes;
        let hash = bytes
            .as_ref()
            .map(|b| policy_compiler::policy_hash(b))
            .unwrap_or_default();
        rows.push(json!({
            "label": label,
            "path": path,
            "required": required,
            "exists": exists,
            "size_bytes": size_bytes,
            "sha256": hash
        }));
    }

    let routes = manifest
        .get("routes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|route| {
            route
                .as_str()
                .map(|path| json!({ "path": path, "declared": true }))
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "manifest_path": MANIFEST,
        "name": manifest.get("name").and_then(Value::as_str).unwrap_or("Submission Audit"),
        "generated_for": manifest.get("generated_for").and_then(Value::as_str).unwrap_or(""),
        "status": if missing_required == 0 { "ready" } else { "missing_required" },
        "summary": {
            "artifacts": rows.len(),
            "present": present,
            "missing_required": missing_required,
            "routes": routes.len(),
            "total_bytes": total_bytes
        },
        "artifacts": rows,
        "routes": routes
    }))
}
