//! x402 settlement support on the TWAK (self-custody) side.
//!
//! When a data provider (e.g. CMC) returns HTTP 402, the client builds an
//! authorization payload; TWAK — which holds the keys — signs it. This module
//! provides the signing entry point. The default in-process signer is a
//! deterministic mock (no real key material); a production deployment routes
//! `sign_authorization` to the TWAK MCP/REST signer.
//!
//! ## Attaching payments during execution
//!
//! Both execution surfaces wire `sign_authorization` into their request paths,
//! mirroring the cmc-client settlement pattern:
//!
//! - [`crate::rest::TwakRestClient`] retries a POST that returns HTTP 402 with
//!   the signed authorization in the `X-PAYMENT` header.
//! - [`crate::mcp::TwakMcpClient`] retries a swap whose JSON-RPC result reports
//!   `payment_required`, carrying the signed authorization in `params`.
//!
//! Because TWAK holds the keys, signing happens in-process here rather than
//! being delegated outward to a separate wallet — TWAK is the sole signer and
//! the sole execution venue.

pub const X402_SUPPORTED: bool = true;

use serde::{Deserialize, Serialize};

/// A signed x402 authorization ready to attach to the `X-PAYMENT` header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAuthorization {
    /// The canonical authorization JSON that was signed.
    pub authorization: String,
    /// Hex signature over `authorization`.
    pub signature: String,
    /// Address that produced the signature.
    pub signer: String,
}

/// Deterministically "sign" an authorization payload.
///
/// The mock signer derives a stable hex signature from the wallet address and
/// the authorization bytes so paper/test flows are reproducible. Swap this for
/// the TWAK signer in production; the call site does not change.
pub fn sign_authorization(authorization: &str, signer: &str) -> SignedAuthorization {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(signer.as_bytes());
    hasher.update(b"\x00");
    hasher.update(authorization.as_bytes());
    let signature = format!("0x{}", hex::encode(hasher.finalize()));
    SignedAuthorization {
        authorization: authorization.to_string(),
        signature,
        signer: signer.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTH: &str = r#"{"amount":"1000000","from":"0xPayer"}"#;
    const SIGNER: &str = "0x000000000000000000000000000000000000dEaD";

    #[test]
    fn sign_authorization_is_deterministic() {
        let a = sign_authorization(AUTH, SIGNER);
        let b = sign_authorization(AUTH, SIGNER);
        assert_eq!(a.signature, b.signature);
        // Round-tripped inputs are preserved on the result.
        assert_eq!(a.authorization, AUTH);
        assert_eq!(a.signer, SIGNER);
    }

    #[test]
    fn different_signer_changes_signature() {
        let a = sign_authorization(AUTH, SIGNER);
        let b = sign_authorization(AUTH, "0xAnotherSignerAddress");
        assert_ne!(a.signature, b.signature);
    }

    #[test]
    fn different_authorization_changes_signature() {
        let a = sign_authorization(AUTH, SIGNER);
        let b = sign_authorization(r#"{"amount":"2000000","from":"0xPayer"}"#, SIGNER);
        assert_ne!(a.signature, b.signature);
    }

    #[test]
    fn signature_has_expected_hex_shape() {
        let signed = sign_authorization(AUTH, SIGNER);
        assert!(
            signed.signature.starts_with("0x"),
            "signature must be 0x-prefixed: {}",
            signed.signature
        );
        // "0x" + 64 hex chars (SHA-256 = 32 bytes).
        assert_eq!(signed.signature.len(), 2 + 64);
        let hex_part = &signed.signature[2..];
        assert!(
            hex_part.chars().all(|c| c.is_ascii_hexdigit()),
            "signature body must be hex: {hex_part}"
        );
    }
}
