//! CLI execution surface for the Trust Wallet Agent Kit.
//!
//! `TwakCliClient` shells out to the `twak` binary for the operations that are
//! naturally interactive or one-shot: competition registration and reading the
//! wallet address. Swaps are intentionally not routed through the CLI — those
//! should use the REST or MCP surfaces, so the swap methods return
//! [`TwakError::Rejected`]. The synchronous `Command` calls are run with bounded
//! output and never block on stdin.

use crate::error::TwakError;
use crate::parse;
use crate::portfolio::TwakPortfolio;
use crate::quote::SwapQuote;
use crate::swap::TxReceipt;
use crate::TwakExecutor;
use async_trait::async_trait;
use common::{Address, OrderIntent};
use risk_engine::ApprovedOrder;
use std::process::Command;

/// Default `twak` binary name (resolved via `PATH`).
pub const TWAK_CLI: &str = "twak";

/// Client that drives the `twak` CLI for registration and wallet lookup.
#[derive(Debug, Clone)]
pub struct TwakCliClient {
    bin: String,
}

impl Default for TwakCliClient {
    fn default() -> Self {
        TwakCliClient {
            bin: TWAK_CLI.to_string(),
        }
    }
}

impl TwakCliClient {
    /// Build a CLI client invoking `bin` (defaults to `"twak"`).
    pub fn new(bin: impl Into<String>) -> Self {
        TwakCliClient { bin: bin.into() }
    }

    /// Run `twak <args...>` and return trimmed stdout, mapping non-zero exits
    /// and spawn failures to [`TwakError`].
    fn run(&self, args: &[&str]) -> Result<String, TwakError> {
        let output = Command::new(&self.bin)
            .args(args)
            .output()
            .map_err(|e| TwakError::Transport(format!("failed to run `{}`: {e}", self.bin)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(TwakError::Rejected(format!(
                "`{} {}` failed: {}",
                self.bin,
                args.join(" "),
                stderr.trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[async_trait]
impl TwakExecutor for TwakCliClient {
    async fn wallet_address(&self) -> Result<Address, TwakError> {
        let out = self.run(&["wallet", "address"])?;
        // Accept either a bare address line or a JSON blob.
        let addr = serde_json::from_str::<serde_json::Value>(&out)
            .ok()
            .and_then(|v| parse::wallet_address(&v))
            .unwrap_or_else(|| out.clone());
        if addr.is_empty() {
            return Err(TwakError::Parse("empty wallet address from CLI".into()));
        }
        Ok(Address::new(addr))
    }

    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError> {
        Err(TwakError::Rejected("use REST/MCP for portfolio".into()))
    }

    async fn quote_swap(&self, _intent: &OrderIntent) -> Result<SwapQuote, TwakError> {
        Err(TwakError::Rejected("use REST/MCP for swaps".into()))
    }

    async fn execute_swap(&self, _approved: &ApprovedOrder) -> Result<TxReceipt, TwakError> {
        Err(TwakError::Rejected("use REST/MCP for swaps".into()))
    }

    async fn register_competition(&self) -> Result<TxReceipt, TwakError> {
        let out = self.run(&["compete", "register"])?;
        let receipt = serde_json::from_str::<serde_json::Value>(&out)
            .map(|v| parse::tx_receipt(&v))
            .unwrap_or_else(|_| TxReceipt {
                tx_hash: out,
                status: "submitted".to_string(),
                block_number: None,
            });
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_twak_binary() {
        let c = TwakCliClient::default();
        assert_eq!(c.bin, "twak");
    }

    #[test]
    fn new_overrides_binary() {
        let c = TwakCliClient::new("/usr/local/bin/twak");
        assert_eq!(c.bin, "/usr/local/bin/twak");
    }

    #[tokio::test]
    async fn missing_binary_is_transport_error() {
        let c = TwakCliClient::new("twak-binary-that-does-not-exist-xyz");
        let err = c.register_competition().await.expect_err("should fail");
        assert!(matches!(err, TwakError::Transport(_)));
    }
}
