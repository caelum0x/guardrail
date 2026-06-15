//! Off-chain agent metadata.
//!
//! [`AgentMetadata`] is the JSON document an agent publishes (typically at the
//! `metadata_url` of its [`crate::identity::AgentIdentity`]). It commits to the
//! agent's trading strategy and policy by including their SHA-256 hashes.

use serde::{Deserialize, Serialize};

use crate::error::AgentError;

/// Serializable off-chain metadata describing a BNB AI-Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Human-readable agent name.
    pub name: String,
    /// Free-form description of the agent's purpose.
    pub description: String,
    /// SHA-256 hex digest committing to the agent's strategy artifact.
    pub strategy_hash: String,
    /// SHA-256 hex digest committing to the agent's policy artifact.
    pub policy_hash: String,
    /// Semantic version of the agent.
    pub version: String,
}

impl AgentMetadata {
    /// Serializes the metadata document to a compact JSON string.
    pub fn to_json(&self) -> Result<String, AgentError> {
        serde_json::to_string(self).map_err(AgentError::from)
    }

    /// Serializes the metadata document to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, AgentError> {
        serde_json::to_string_pretty(self).map_err(AgentError::from)
    }

    /// Parses an [`AgentMetadata`] document from a JSON string.
    pub fn from_json(json: &str) -> Result<Self, AgentError> {
        serde_json::from_str(json).map_err(AgentError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> AgentMetadata {
        AgentMetadata {
            name: "guardrail".into(),
            description: "Risk-guarded trading agent".into(),
            strategy_hash: "aa".repeat(32),
            policy_hash: "bb".repeat(32),
            version: "1.0.0".into(),
        }
    }

    #[test]
    fn json_round_trips() {
        let meta = sample();
        let json = meta.to_json().expect("serialize");
        let back = AgentMetadata::from_json(&json).expect("deserialize");
        assert_eq!(meta, back);
    }

    #[test]
    fn json_contains_commitment_hashes() {
        let json = sample().to_json().expect("serialize");
        assert!(json.contains("strategy_hash"));
        assert!(json.contains("policy_hash"));
    }
}
