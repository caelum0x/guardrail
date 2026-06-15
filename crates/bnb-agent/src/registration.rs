//! Deterministic registration request/receipt artifacts.
//!
//! No chain calls are made. [`build_registration_request`] assembles the payload
//! that *would* be submitted to a registry, and derives a deterministic
//! `registration_id` by hashing that payload. A [`RegistrationReceipt`] records
//! the outcome, optionally carrying a transaction hash supplied out-of-band.

use serde::{Deserialize, Serialize};

use crate::identity::AgentIdentity;
use crate::metadata::AgentMetadata;
use crate::report_hash::sha256_hex;

/// Payload that would be submitted to register an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationRequest {
    /// Deterministic agent identifier.
    pub agent_id: String,
    /// Agent name.
    pub name: String,
    /// Owner wallet address.
    pub wallet_address: String,
    /// Optional off-chain metadata URI.
    pub metadata_url: Option<String>,
    /// SHA-256 commitment to the agent strategy.
    pub strategy_hash: String,
    /// SHA-256 commitment to the agent policy.
    pub policy_hash: String,
    /// Semantic version of the agent.
    pub version: String,
}

impl RegistrationRequest {
    /// Derives the deterministic registration id (lowercase hex SHA-256) by
    /// hashing the canonical JSON encoding of this request.
    ///
    /// Serialization of this struct is field-ordered and stable, so identical
    /// requests always produce the same id without any chain interaction.
    pub fn registration_id(&self) -> String {
        match serde_json::to_vec(self) {
            Ok(bytes) => sha256_hex(&bytes),
            // Serializing a plain struct of strings is infallible in practice;
            // fall back to hashing the debug form rather than panicking.
            Err(_) => sha256_hex(format!("{self:?}").as_bytes()),
        }
    }
}

/// Outcome of a registration attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationReceipt {
    /// Deterministic id of the registration request.
    pub registration_id: String,
    /// Agent identifier the receipt is for.
    pub agent_id: String,
    /// Optional transaction hash, if the record was later anchored on-chain.
    pub registration_tx: Option<String>,
}

/// Builds a [`RegistrationRequest`] from an identity and its metadata.
pub fn build_registration_request(
    identity: &AgentIdentity,
    metadata: &AgentMetadata,
) -> RegistrationRequest {
    RegistrationRequest {
        agent_id: identity.agent_id(),
        name: identity.name.clone(),
        wallet_address: identity.wallet_address.clone(),
        metadata_url: identity.metadata_url.clone(),
        strategy_hash: metadata.strategy_hash.clone(),
        policy_hash: metadata.policy_hash.clone(),
        version: metadata.version.clone(),
    }
}

/// Builds a [`RegistrationReceipt`] for a request, optionally with a tx hash.
pub fn build_registration_receipt(
    request: &RegistrationRequest,
    registration_tx: Option<String>,
) -> RegistrationReceipt {
    RegistrationReceipt {
        registration_id: request.registration_id(),
        agent_id: request.agent_id.clone(),
        registration_tx,
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
    fn registration_id_is_deterministic() {
        let (identity, metadata) = fixtures();
        let req = build_registration_request(&identity, &metadata);
        assert_eq!(req.registration_id(), req.registration_id());
        assert_eq!(req.registration_id().len(), 64);
    }

    #[test]
    fn registration_id_changes_with_policy_hash() {
        let (identity, mut metadata) = fixtures();
        let a = build_registration_request(&identity, &metadata).registration_id();
        metadata.policy_hash = "cc".repeat(32);
        let b = build_registration_request(&identity, &metadata).registration_id();
        assert_ne!(a, b);
    }

    #[test]
    fn receipt_carries_request_fields() {
        let (identity, metadata) = fixtures();
        let req = build_registration_request(&identity, &metadata);
        let receipt = build_registration_receipt(&req, Some("0xdeadbeef".into()));
        assert_eq!(receipt.registration_id, req.registration_id());
        assert_eq!(receipt.agent_id, identity.agent_id());
        assert_eq!(receipt.registration_tx.as_deref(), Some("0xdeadbeef"));
    }
}
