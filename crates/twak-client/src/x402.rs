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

/// Env selecting the signing backend: `bnb-sdk` routes to the real
/// EIP-712/EIP-3009 signer; anything else (or unset) uses the offline mock.
pub const SIGNER_ENV: &str = "X402_SIGNER";
/// Env carrying the payee the caller commits to (anti-MITM guard for the real
/// signer). Sourced independently of the 402 challenge body.
pub const EXPECTED_TO_ENV: &str = "X402_EXPECTED_TO";
/// Env overriding the signer command (default: the bundled Python helper).
pub const SIGNER_CMD_ENV: &str = "X402_SIGNER_CMD";

/// Default command that runs the real signer helper.
const DEFAULT_SIGNER_CMD: &str = "python3 integrations/x402-signer/x402_sign.py";

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};

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

/// Sign an x402 challenge, dispatching to the real BNB-SDK signer when
/// `X402_SIGNER=bnb-sdk`, otherwise the deterministic offline mock.
///
/// `challenge_json` is the raw x402 v2 body from the 402 response. The real
/// path is gated: it runs only when explicitly enabled, a committed payee is
/// available (`X402_EXPECTED_TO`), and the helper finds a `WALLET_PASSWORD`. Any
/// failure falls back to the mock so a misconfigured live run cannot stall —
/// and never signs a real spend.
pub fn sign_challenge(challenge_json: &str, signer_hint: &str) -> SignedAuthorization {
    let use_real = std::env::var(SIGNER_ENV)
        .map(|v| v.eq_ignore_ascii_case("bnb-sdk"))
        .unwrap_or(false);
    if !use_real {
        return sign_authorization(challenge_json, signer_hint);
    }
    match sign_via_bnb_sdk(challenge_json) {
        Ok(signed) => signed,
        Err(e) => {
            tracing::warn!(error = %e, "x402 bnb-sdk signer failed; falling back to mock");
            sign_authorization(challenge_json, signer_hint)
        }
    }
}

/// Shell out to the BNB-SDK signer helper, piping the challenge to stdin and
/// parsing its `{signature, from, envelope}` JSON into a [`SignedAuthorization`].
fn sign_via_bnb_sdk(challenge_json: &str) -> Result<SignedAuthorization, String> {
    let expected_to = std::env::var(EXPECTED_TO_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("${EXPECTED_TO_ENV} not set (committed payee required)"))?;

    let cmd = std::env::var(SIGNER_CMD_ENV).unwrap_or_else(|_| DEFAULT_SIGNER_CMD.to_string());
    let mut parts = cmd.split_whitespace();
    let program = parts.next().ok_or("empty signer command")?;
    let prog_args: Vec<&str> = parts.collect();

    let mut child = Command::new(program)
        .args(&prog_args)
        .arg("--expected-to")
        .arg(&expected_to)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn signer `{program}`: {e}"))?;

    child
        .stdin
        .take()
        .ok_or("no stdin handle")?
        .write_all(challenge_json.as_bytes())
        .map_err(|e| format!("failed writing challenge to signer: {e}"))?;

    let out = child
        .wait_with_output()
        .map_err(|e| format!("signer wait failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "signer exited {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("signer output not JSON: {e}"))?;
    let signature = v
        .get("signature")
        .and_then(|s| s.as_str())
        .ok_or("signer output missing `signature`")?
        .to_string();
    let signer = v
        .get("from")
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_string();
    // Prefer the ready-to-send X-PAYMENT envelope as the authorization blob.
    let authorization = v
        .get("envelope")
        .and_then(|s| s.as_str())
        .unwrap_or(challenge_json)
        .to_string();
    Ok(SignedAuthorization {
        authorization,
        signature,
        signer,
    })
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
    fn sign_challenge_falls_back_to_mock_without_env() {
        std::env::remove_var(SIGNER_ENV);
        let viaq = sign_challenge(AUTH, SIGNER);
        let mock = sign_authorization(AUTH, SIGNER);
        assert_eq!(viaq.signature, mock.signature);
        assert_eq!(viaq.signer, SIGNER);
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
