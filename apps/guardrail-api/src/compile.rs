//! Natural-language policy compiler endpoint.
//!
//! Accepts a free-text mandate, compiles it into a validated `RiskPolicy` via
//! the `policy-compiler` crate, and returns the canonical hash alongside the
//! resulting policy as JSON. Read-only and side-effect free.

use axum::extract::Query;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
pub struct CompileParams {
    /// Free-text mandate to compile (required, non-empty).
    pub mandate: Option<String>,
}

pub async fn compile(Query(params): Query<CompileParams>) -> Json<Value> {
    let mandate = params.mandate.unwrap_or_default();
    let trimmed = mandate.trim();
    if trimmed.is_empty() {
        return Json(json!({ "error": "mandate required" }));
    }

    match policy_compiler::compile_mandate(trimmed) {
        Ok(compiled) => match policy_to_value(&compiled.policy) {
            Ok(policy) => Json(json!({
                "hash": compiled.hash,
                "policy": policy,
            })),
            Err(error) => Json(json!({ "error": error.to_string() })),
        },
        Err(error) => Json(json!({ "error": error.to_string() })),
    }
}

/// Convert the compiled policy into a JSON value, preferring the compiler's
/// canonical string form and falling back to direct serialization.
fn policy_to_value(policy: &risk_engine::RiskPolicy) -> anyhow::Result<Value> {
    match policy_compiler::compiler::policy_to_json(policy) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(_) => Ok(serde_json::to_value(policy)?),
    }
}
