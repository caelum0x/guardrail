//! ERC-8004 agent registration record.
//!
//! This struct is a typed, off-chain mirror of the registry record shape defined
//! by the ERC-8004 "AI Agent Identity" standard. It is NOT a chain transaction:
//! it captures exactly the fields that would be written to the on-chain registry,
//! so they can be hashed, signed, or shown to a judge deterministically.

use serde::{Deserialize, Serialize};

use crate::identity::AgentIdentity;
use crate::metadata::AgentMetadata;

/// Marker for the supported ERC-8004 schema version of this record.
pub const ERC8004_VERSION: &str = "erc8004:1";

/// Typed mirror of an ERC-8004 on-chain registry record.
///
/// Field names mirror the registry record entries (agent id, owner wallet,
/// metadata pointer, and the strategy/policy commitments) rather than ABI types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Erc8004Record {
    /// Schema discriminator, always [`ERC8004_VERSION`].
    pub schema: String,
    /// Deterministic agent identifier (SHA-256 of name + wallet).
    pub agent_id: String,
    /// Owner wallet address that controls the agent record.
    pub owner: String,
    /// Optional URI pointing at the off-chain metadata document.
    pub metadata_uri: Option<String>,
    /// SHA-256 commitment to the agent strategy.
    pub strategy_hash: String,
    /// SHA-256 commitment to the agent policy.
    pub policy_hash: String,
    /// Semantic version of the agent.
    pub version: String,
}

impl Erc8004Record {
    /// Builds the registry record from an [`AgentIdentity`] and [`AgentMetadata`].
    pub fn build(identity: &AgentIdentity, metadata: &AgentMetadata) -> Self {
        Self {
            schema: ERC8004_VERSION.to_string(),
            agent_id: identity.agent_id(),
            owner: identity.wallet_address.clone(),
            metadata_uri: identity.metadata_url.clone(),
            strategy_hash: metadata.strategy_hash.clone(),
            policy_hash: metadata.policy_hash.clone(),
            version: metadata.version.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (AgentIdentity, AgentMetadata) {
        let identity = AgentIdentity::new("guardrail", "0xabc123")
            .with_metadata_url("https://example.com/meta.json");
        let metadata = AgentMetadata {
            name: "guardrail".into(),
            description: "desc".into(),
            strategy_hash: "aa".repeat(32),
            policy_hash: "bb".repeat(32),
            version: "1.0.0".into(),
        };
        (identity, metadata)
    }

    #[test]
    fn build_mirrors_identity_and_metadata() {
        let (identity, metadata) = fixtures();
        let record = Erc8004Record::build(&identity, &metadata);
        assert_eq!(record.schema, ERC8004_VERSION);
        assert_eq!(record.agent_id, identity.agent_id());
        assert_eq!(record.owner, "0xabc123");
        assert_eq!(
            record.metadata_uri.as_deref(),
            Some("https://example.com/meta.json")
        );
        assert_eq!(record.policy_hash, metadata.policy_hash);
    }

    #[test]
    fn build_is_deterministic() {
        let (identity, metadata) = fixtures();
        assert_eq!(
            Erc8004Record::build(&identity, &metadata),
            Erc8004Record::build(&identity, &metadata)
        );
    }
}
