//! Agent identity primitives.
//!
//! An [`AgentIdentity`] binds a human-readable agent name to a BNB-chain wallet
//! address and an optional off-chain metadata URL. The derived `agent_id` is a
//! deterministic SHA-256 digest of the name and wallet, so the same inputs always
//! yield the same identifier without any chain interaction.

use serde::{Deserialize, Serialize};

use crate::report_hash::sha256_hex;

/// Identity of a BNB AI-Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentIdentity {
    /// Human-readable agent name.
    pub name: String,
    /// BNB-chain wallet address (hex string, e.g. `0x...`).
    pub wallet_address: String,
    /// Optional URL pointing at off-chain agent metadata.
    pub metadata_url: Option<String>,
}

impl AgentIdentity {
    /// Creates a new identity from a name and wallet address.
    pub fn new(name: impl Into<String>, wallet_address: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            wallet_address: wallet_address.into(),
            metadata_url: None,
        }
    }

    /// Returns a copy of this identity with the given metadata URL attached.
    pub fn with_metadata_url(self, metadata_url: impl Into<String>) -> Self {
        Self {
            metadata_url: Some(metadata_url.into()),
            ..self
        }
    }

    /// Deterministic agent identifier: lowercase hex SHA-256 of `name` + `wallet`.
    ///
    /// The two fields are joined with a `\0` separator so that distinct
    /// `(name, wallet)` pairs cannot collide via boundary ambiguity.
    pub fn agent_id(&self) -> String {
        let mut preimage = Vec::with_capacity(self.name.len() + self.wallet_address.len() + 1);
        preimage.extend_from_slice(self.name.as_bytes());
        preimage.push(0);
        preimage.extend_from_slice(self.wallet_address.as_bytes());
        sha256_hex(&preimage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_id_is_deterministic() {
        let a = AgentIdentity::new("guardrail", "0xabc123");
        let b = AgentIdentity::new("guardrail", "0xabc123");
        assert_eq!(a.agent_id(), b.agent_id());
    }

    #[test]
    fn agent_id_is_64_hex_chars() {
        let id = AgentIdentity::new("guardrail", "0xabc123").agent_id();
        assert_eq!(id.len(), 64);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn distinct_inputs_yield_distinct_ids() {
        let a = AgentIdentity::new("guardrail", "0xabc123").agent_id();
        let b = AgentIdentity::new("guardrail", "0xabc124").agent_id();
        let c = AgentIdentity::new("guardrai", "l0xabc123").agent_id();
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn metadata_url_does_not_affect_agent_id() {
        let base = AgentIdentity::new("guardrail", "0xabc123");
        let with_url = base
            .clone()
            .with_metadata_url("https://example.com/meta.json");
        assert_eq!(base.agent_id(), with_url.agent_id());
        assert_eq!(
            with_url.metadata_url.as_deref(),
            Some("https://example.com/meta.json")
        );
    }
}
