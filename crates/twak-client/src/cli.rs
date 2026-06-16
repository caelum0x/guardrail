//! CLI execution surface for the Trust Wallet Agent Kit.
//!
//! `TwakCliClient` shells out to the real `twak` binary (v0.19+) for the full
//! self-custody trade loop: reading the wallet address, quoting and executing
//! swaps on BSC, and registering the competition wallet. TWAK holds the keys
//! and is the sole signer — this client never sees key material; it passes the
//! wallet password through to `twak` only when an already-risk-approved order
//! is executed under autonomous mode.
//!
//! ## Money-action gating
//!
//! - `quote_swap` is read-only (`--quote-only`) and needs no password.
//! - `execute_swap` spends real funds. It is refused unless the client is in
//!   autonomous mode AND a wallet password is available in the environment, so
//!   a misconfigured paper run can never sign a live swap. The upstream
//!   `risk-engine` remains the gate that produces the `ApprovedOrder`.
//!
//! Every invocation passes `--json`; `twak` reports failures as a
//! `{ "error": ..., "errorCode": ... }` envelope (e.g. missing
//! `TWAK_ACCESS_ID`/`TWAK_HMAC_SECRET`), which is mapped to [`TwakError`].

use crate::error::TwakError;
use crate::identity::Erc8004Identity;
use crate::parse;
use crate::portfolio::TwakPortfolio;
use crate::quote::SwapQuote;
use crate::swap::TxReceipt;
use crate::TwakExecutor;
use async_trait::async_trait;
use common::{Address, OrderIntent};
use risk_engine::ApprovedOrder;
use serde_json::Value;
use std::process::Command;

/// Default `twak` binary name (resolved via `PATH`).
pub const TWAK_CLI: &str = "twak";

/// Default BSC chain selector understood by the `twak` CLI.
pub const DEFAULT_CHAIN: &str = "bsc";

/// Environment variable holding the wallet password used to sign live swaps.
pub const PASSWORD_ENV: &str = "TWAK_WALLET_PASSWORD";

/// Default swap slippage tolerance (percent), as a CLI string.
pub const DEFAULT_SLIPPAGE_PCT: &str = "1";

/// Client that drives the `twak` CLI for the full trade loop.
#[derive(Debug, Clone)]
pub struct TwakCliClient {
    bin: String,
    chain: String,
    slippage_pct: String,
    autonomous: bool,
    password_env: String,
}

impl Default for TwakCliClient {
    fn default() -> Self {
        TwakCliClient {
            bin: TWAK_CLI.to_string(),
            chain: DEFAULT_CHAIN.to_string(),
            slippage_pct: DEFAULT_SLIPPAGE_PCT.to_string(),
            autonomous: false,
            password_env: PASSWORD_ENV.to_string(),
        }
    }
}

impl TwakCliClient {
    /// Build a CLI client invoking `bin` (defaults to `"twak"`).
    pub fn new(bin: impl Into<String>) -> Self {
        TwakCliClient {
            bin: bin.into(),
            ..Self::default()
        }
    }

    /// Set the chain selector passed to `twak` (defaults to `"bsc"`).
    pub fn with_chain(mut self, chain: impl Into<String>) -> Self {
        self.chain = chain.into();
        self
    }

    /// Enable autonomous execution — required (with a password) for live swaps.
    pub fn with_autonomous(mut self, autonomous: bool) -> Self {
        self.autonomous = autonomous;
        self
    }

    /// Override the slippage tolerance (percent) used when quoting/executing.
    pub fn with_slippage_pct(mut self, slippage_pct: impl Into<String>) -> Self {
        self.slippage_pct = slippage_pct.into();
        self
    }

    /// Whether this client may self-submit (sign) swaps.
    pub fn is_autonomous(&self) -> bool {
        self.autonomous
    }

    /// The wallet password from the environment, if present and non-empty.
    fn password(&self) -> Option<String> {
        std::env::var(&self.password_env)
            .ok()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
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

    /// Run `twak <args...> --json` and parse stdout as JSON, surfacing the
    /// `{ "error", "errorCode" }` envelope `twak` emits on failure (e.g. missing
    /// API credentials) as [`TwakError::Rejected`].
    fn run_json(&self, args: &[&str]) -> Result<Value, TwakError> {
        let out = self.run(args)?;
        let v: Value =
            serde_json::from_str(&out).map_err(|e| TwakError::Parse(format!("{e}: {out}")))?;
        if let Some(err) = v.get("error").and_then(Value::as_str) {
            let code = v
                .get("errorCode")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN");
            return Err(TwakError::Rejected(format!("twak {code}: {err}")));
        }
        Ok(v)
    }

    /// Read the on-chain state of an ERC-8004 agent identity (read-only).
    pub async fn erc8004_show(&self, agent_id: &str) -> Result<Erc8004Identity, TwakError> {
        let v = self.run_json(&["erc8004", "show", agent_id, "--chain", &self.chain, "--json"])?;
        Ok(Erc8004Identity::from_json(&v))
    }

    /// Mint a new ERC-8004 agent identity NFT (money/irreversible on-chain
    /// write). Gated identically to swap execution: refused unless the client
    /// is autonomous AND a wallet password is available. `metadata` entries are
    /// passed as repeatable `--metadata key=value` flags.
    pub async fn erc8004_register(
        &self,
        uri: &str,
        metadata: &[(String, String)],
    ) -> Result<Erc8004Identity, TwakError> {
        if !self.autonomous {
            return Err(TwakError::Rejected(
                "erc8004 register refused: client is not in autonomous mode".into(),
            ));
        }
        let Some(password) = self.password() else {
            return Err(TwakError::Rejected(format!(
                "erc8004 register refused: no wallet password in ${} (on-chain mint gated)",
                self.password_env
            )));
        };

        let mut args: Vec<String> = vec![
            "erc8004".into(),
            "register".into(),
            "--uri".into(),
            uri.to_string(),
            "--chain".into(),
            self.chain.clone(),
            "--password".into(),
            password,
            "--json".into(),
        ];
        for (k, val) in metadata {
            args.push("--metadata".into());
            args.push(format!("{k}={val}"));
        }
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let v = self.run_json(&arg_refs)?;
        Ok(Erc8004Identity::from_json(&v))
    }
}

#[async_trait]
impl TwakExecutor for TwakCliClient {
    async fn wallet_address(&self) -> Result<Address, TwakError> {
        // `twak wallet address --chain <c> --json` → { address: ... } | bare addr.
        let v = self.run_json(&["wallet", "address", "--chain", &self.chain, "--json"])?;
        let addr = parse::wallet_address(&v)
            .ok_or_else(|| TwakError::Parse("empty wallet address from CLI".into()))?;
        if addr.is_empty() {
            return Err(TwakError::Parse("empty wallet address from CLI".into()));
        }
        Ok(Address::new(addr))
    }

    async fn portfolio(&self) -> Result<TwakPortfolio, TwakError> {
        // `twak wallet portfolio --json` → { balances: [...] } | [...].
        let v = self.run_json(&["wallet", "portfolio", "--json"])?;
        Ok(parse::portfolio(&v))
    }

    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote, TwakError> {
        // Read-only quote: swap a USD-equivalent of the source token, no signing.
        let amount = intent.amount_usd.normalize().to_string();
        let v = self.run_json(&[
            "swap",
            &intent.from_symbol,
            &intent.to_symbol,
            "--usd",
            &amount,
            "--chain",
            &self.chain,
            "--slippage",
            &self.slippage_pct,
            "--quote-only",
            "--json",
        ])?;
        parse::swap_quote(&v, intent)
            .ok_or_else(|| TwakError::Parse("malformed swap quote from CLI".into()))
    }

    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt, TwakError> {
        // Money action: refuse unless explicitly autonomous AND a password is
        // available. The order is already risk-approved upstream; this is the
        // final self-custody spend gate.
        if !self.autonomous {
            return Err(TwakError::Rejected(
                "CLI execute refused: order approved but client is not in autonomous mode".into(),
            ));
        }
        let Some(password) = self.password() else {
            return Err(TwakError::Rejected(format!(
                "CLI execute refused: no wallet password in ${} (live signing gated)",
                self.password_env
            )));
        };

        let amount = approved.approved_amount_usd.normalize().to_string();
        let v = self.run_json(&[
            "swap",
            &approved.intent.from_symbol,
            &approved.intent.to_symbol,
            "--usd",
            &amount,
            "--chain",
            &self.chain,
            "--slippage",
            &self.slippage_pct,
            "--password",
            &password,
            "--json",
        ])?;
        Ok(parse::tx_receipt(&v))
    }

    async fn register_competition(&self) -> Result<TxReceipt, TwakError> {
        let v = self.run_json(&["compete", "register", "--json"])?;
        Ok(parse::tx_receipt(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_twak_binary_and_bsc() {
        let c = TwakCliClient::default();
        assert_eq!(c.bin, "twak");
        assert_eq!(c.chain, "bsc");
        assert!(!c.is_autonomous());
    }

    #[test]
    fn new_overrides_binary() {
        let c = TwakCliClient::new("/usr/local/bin/twak");
        assert_eq!(c.bin, "/usr/local/bin/twak");
    }

    #[test]
    fn builders_set_chain_and_autonomy() {
        let c = TwakCliClient::new("twak")
            .with_chain("ethereum")
            .with_autonomous(true);
        assert_eq!(c.chain, "ethereum");
        assert!(c.is_autonomous());
    }

    #[tokio::test]
    async fn missing_binary_is_transport_error() {
        let c = TwakCliClient::new("twak-binary-that-does-not-exist-xyz");
        let err = c.register_competition().await.expect_err("should fail");
        assert!(matches!(err, TwakError::Transport(_)));
    }

    #[tokio::test]
    async fn execute_refused_when_not_autonomous() {
        use common::{Decimal, OrderIntent, OrderSide};
        use risk_engine::{ApprovedOrder, RiskDecision};
        let intent = OrderIntent::new(OrderSide::Buy, "USDT", "CAKE", Decimal::from(10), "t");
        let approved = ApprovedOrder {
            id: "o1".into(),
            intent: intent.clone(),
            approved_amount_usd: Decimal::from(10),
            decision: RiskDecision::Approved,
        };
        // Not autonomous → refused before any subprocess runs.
        let c = TwakCliClient::new("twak-binary-that-does-not-exist-xyz");
        let err = c.execute_swap(&approved).await.expect_err("gated");
        assert!(matches!(err, TwakError::Rejected(_)));
    }

    #[tokio::test]
    async fn erc8004_register_refused_when_not_autonomous() {
        let c = TwakCliClient::new("twak-binary-that-does-not-exist-xyz");
        let err = c
            .erc8004_register("https://x/agent.json", &[])
            .await
            .expect_err("gated");
        assert!(matches!(err, TwakError::Rejected(_)));
    }
}
