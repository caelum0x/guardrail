//! Judge-facing proof artifact.
//!
//! [`AgentProof`] bundles the fields a reviewer needs to verify an agent's
//! identity and report commitment, plus helpers that format BscScan explorer
//! URLs for the registration transaction and the agent wallet address. No
//! network access occurs; the URLs are purely string-formatted links.

use serde::{Deserialize, Serialize};

/// BscScan base URL used for explorer links.
pub const BSCSCAN_BASE_URL: &str = "https://bscscan.com";

/// Judge-facing proof of an agent's identity and report commitment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProof {
    /// Deterministic agent identifier.
    pub agent_id: String,
    /// Agent wallet address.
    pub wallet_address: String,
    /// Optional registration transaction hash (anchored out-of-band).
    pub registration_tx: Option<String>,
    /// SHA-256 commitment to the agent policy.
    pub policy_hash: String,
    /// SHA-256 commitment to the submitted report.
    pub report_hash: String,
}

impl AgentProof {
    /// Creates a new proof artifact.
    pub fn new(
        agent_id: impl Into<String>,
        wallet_address: impl Into<String>,
        policy_hash: impl Into<String>,
        report_hash: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            wallet_address: wallet_address.into(),
            registration_tx: None,
            policy_hash: policy_hash.into(),
            report_hash: report_hash.into(),
        }
    }

    /// Returns a copy of this proof with a registration transaction hash set.
    pub fn with_registration_tx(self, tx: impl Into<String>) -> Self {
        Self {
            registration_tx: Some(tx.into()),
            ..self
        }
    }

    /// Formats a BscScan URL for the registration transaction.
    ///
    /// Returns `None` when no `registration_tx` is present.
    pub fn tx_url(&self) -> Option<String> {
        self.registration_tx
            .as_ref()
            .map(|tx| format!("{BSCSCAN_BASE_URL}/tx/{tx}"))
    }

    /// Formats a BscScan URL for the agent wallet address.
    pub fn address_url(&self) -> String {
        format!("{BSCSCAN_BASE_URL}/address/{}", self.wallet_address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_url_is_formatted() {
        let proof = AgentProof::new("agent", "0xabc123", "bb", "cc");
        assert_eq!(proof.address_url(), "https://bscscan.com/address/0xabc123");
    }

    #[test]
    fn tx_url_is_none_without_tx() {
        let proof = AgentProof::new("agent", "0xabc123", "bb", "cc");
        assert_eq!(proof.tx_url(), None);
    }

    #[test]
    fn tx_url_is_formatted_when_present() {
        let proof =
            AgentProof::new("agent", "0xabc123", "bb", "cc").with_registration_tx("0xdeadbeef");
        assert_eq!(
            proof.tx_url(),
            Some("https://bscscan.com/tx/0xdeadbeef".to_string())
        );
    }
}
