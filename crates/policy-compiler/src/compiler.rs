use risk_engine::RiskPolicy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPolicy {
    pub policy: RiskPolicy,
    pub hash: String,
}

/// Compile an existing JSON policy: parse, validate, and hash.
pub fn compile_json_policy(input: &str) -> anyhow::Result<CompiledPolicy> {
    let policy = RiskPolicy::from_json_str(input)?;
    crate::validator::validate_policy(&policy)?;
    let hash = crate::policy_hash::policy_hash(input.as_bytes());
    Ok(CompiledPolicy { policy, hash })
}

/// Compile a natural-language mandate into a validated, hashed policy.
///
/// Pipeline: free text -> heuristic parse -> `RiskPolicy` -> validation ->
/// canonical JSON -> SHA-256 hash. The hash is the on-chain-publishable
/// fingerprint of exactly what binds the runtime.
pub fn compile_mandate(text: &str) -> anyhow::Result<CompiledPolicy> {
    let policy = crate::parser::parse_mandate(text);
    crate::validator::validate_policy(&policy)?;
    let json = serde_json::to_string_pretty(&policy)?;
    let hash = crate::policy_hash::policy_hash(json.as_bytes());
    Ok(CompiledPolicy { policy, hash })
}

/// Serialize a compiled policy to canonical pretty JSON.
pub fn policy_to_json(policy: &RiskPolicy) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(policy)?)
}
