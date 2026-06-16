//! ERC-8004 agent-identity results from the `twak erc8004` surface.
//!
//! TWAK mints and reads the agent's on-chain identity NFT (keys stay in TWAK),
//! so the self-custody signer is also the identity authority. This module holds
//! the parsed result shape; the gated `register` / read-only `show` methods live
//! on [`crate::cli::TwakCliClient`].

use crate::parse;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// On-chain ERC-8004 identity state as reported by `twak erc8004`.
///
/// Fields are optional because `register` and `show` return overlapping but not
/// identical shapes, and TWAK output varies by version; parsing degrades
/// gracefully rather than failing the call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Erc8004Identity {
    /// The minted ERC-721 `agentId`, when present.
    pub agent_id: Option<String>,
    /// The agent registration URI (https/ipfs/data).
    pub agent_uri: Option<String>,
    /// The owning wallet address.
    pub owner: Option<String>,
    /// The mint transaction hash (only on `register`).
    pub tx_hash: Option<String>,
}

impl Erc8004Identity {
    /// Parse a `twak erc8004 register|show --json` response defensively.
    pub fn from_json(v: &Value) -> Self {
        let inner = parse::unwrap_envelope(v);
        Erc8004Identity {
            agent_id: parse::str_at(inner, &["agentId", "agent_id", "id", "tokenId"])
                .map(str::to_string),
            agent_uri: parse::str_at(inner, &["agentURI", "agent_uri", "uri"]).map(str::to_string),
            owner: parse::str_at(inner, &["owner", "address", "wallet"]).map(str::to_string),
            tx_hash: parse::str_at(inner, &["tx_hash", "hash", "transaction_hash", "txid"])
                .map(str::to_string),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_register_shape() {
        let id = Erc8004Identity::from_json(&json!({
            "agentId": "42",
            "agentURI": "https://guardrail/agent.json",
            "hash": "0xabc"
        }));
        assert_eq!(id.agent_id.as_deref(), Some("42"));
        assert_eq!(id.agent_uri.as_deref(), Some("https://guardrail/agent.json"));
        assert_eq!(id.tx_hash.as_deref(), Some("0xabc"));
    }

    #[test]
    fn parses_envelope_and_show_shape() {
        let id = Erc8004Identity::from_json(&json!({
            "result": { "tokenId": "7", "owner": "0xdead" }
        }));
        assert_eq!(id.agent_id.as_deref(), Some("7"));
        assert_eq!(id.owner.as_deref(), Some("0xdead"));
        assert!(id.tx_hash.is_none());
    }
}
