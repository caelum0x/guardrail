//! ERC-8183 agent registration record.
//!
//! This struct is a typed, off-chain mirror of the registry record shape defined
//! by the ERC-8183 standard. As with [`crate::erc8004`], it is NOT a chain
//! transaction: it enumerates the exact fields that would be persisted to the
//! on-chain registry so they can be hashed, signed, or audited deterministically.

use serde::{Deserialize, Serialize};

use crate::identity::AgentIdentity;
use crate::metadata::AgentMetadata;

/// Marker for the supported ERC-8183 schema version of this record.
pub const ERC8183_VERSION: &str = "erc8183:1";

/// Typed mirror of an ERC-8183 on-chain registry record.
///
/// Field names mirror the registry record entries rather than ABI types; the
/// `agent_name` and `controller` fields reflect this standard's naming, while
/// the strategy/policy commitments are shared with the agent metadata document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Erc8183Record {
    /// Schema discriminator, always [`ERC8183_VERSION`].
    pub schema: String,
    /// Deterministic agent identifier (SHA-256 of name + wallet).
    pub agent_id: String,
    /// Registered agent name.
    pub agent_name: String,
    /// Controlling wallet address for the agent record.
    pub controller: String,
    /// Optional URI pointing at the off-chain metadata document.
    pub metadata_uri: Option<String>,
    /// SHA-256 commitment to the agent policy.
    pub policy_hash: String,
    /// Semantic version of the agent.
    pub version: String,
}

impl Erc8183Record {
    /// Builds the registry record from an [`AgentIdentity`] and [`AgentMetadata`].
    pub fn build(identity: &AgentIdentity, metadata: &AgentMetadata) -> Self {
        Self {
            schema: ERC8183_VERSION.to_string(),
            agent_id: identity.agent_id(),
            agent_name: identity.name.clone(),
            controller: identity.wallet_address.clone(),
            metadata_uri: identity.metadata_url.clone(),
            policy_hash: metadata.policy_hash.clone(),
            version: metadata.version.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (AgentIdentity, AgentMetadata) {
        let identity = AgentIdentity::new("guardrail", "0xabc123");
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
        let record = Erc8183Record::build(&identity, &metadata);
        assert_eq!(record.schema, ERC8183_VERSION);
        assert_eq!(record.agent_id, identity.agent_id());
        assert_eq!(record.agent_name, "guardrail");
        assert_eq!(record.controller, "0xabc123");
        assert_eq!(record.policy_hash, metadata.policy_hash);
    }

    #[test]
    fn build_is_deterministic() {
        let (identity, metadata) = fixtures();
        assert_eq!(
            Erc8183Record::build(&identity, &metadata),
            Erc8183Record::build(&identity, &metadata)
        );
    }
}
