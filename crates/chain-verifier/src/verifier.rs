//! Orchestrates the read-only on-chain checks into an [`OnChainReport`].
//!
//! The checks are deliberately **ABI-free**: we never guess a registry view
//! selector. Instead we verify what is universally observable on any EVM chain —
//! the chain id, that the competition contract has deployed bytecode, and that
//! the registration transaction (if anchored) was actually mined to that
//! contract with a success status. This keeps the verification honest: every
//! claim it makes is one a judge can reproduce with `cast`/`curl` against the
//! same RPC.

use serde::Serialize;

use crate::rpc::{BscRpcClient, Receipt};

/// BSC mainnet chain id.
pub const BSC_MAINNET_CHAIN_ID: u64 = 56;

/// Outcome of a single on-chain check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// The check ran and the on-chain state matched the claim.
    Pass,
    /// The check ran and the on-chain state contradicted the claim (or the RPC
    /// call failed). A failing check fails the overall proof.
    Fail,
    /// The check did not run (no RPC configured, or no tx to verify). Skipped
    /// checks never fail the overall proof.
    Skipped,
}

impl CheckStatus {
    /// Lowercase string form, matching the serialized representation and the
    /// `pass`/`fail`/`skipped` vocabulary used by `/proof/verify`.
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Pass => "pass",
            CheckStatus::Fail => "fail",
            CheckStatus::Skipped => "skipped",
        }
    }
}

/// One named on-chain check with a human-readable detail.
#[derive(Debug, Clone, Serialize)]
pub struct OnChainCheck {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
}

/// The full set of on-chain checks plus whether an RPC endpoint was configured.
#[derive(Debug, Clone, Serialize)]
pub struct OnChainReport {
    /// True when a non-empty `BSC_RPC_URL` (or explicit url) was supplied.
    pub configured: bool,
    pub checks: Vec<OnChainCheck>,
}

/// Reads `BSC_RPC_URL`, treating empty/whitespace as unset.
pub fn rpc_url_from_env() -> Option<String> {
    std::env::var("BSC_RPC_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Runs the read-only on-chain checks.
///
/// `rpc_url` of `None`/empty yields a single `Skipped` check and
/// `configured = false` — the offline path. Otherwise each RPC failure becomes
/// its own `Fail` check; the function itself is infallible by design so callers
/// never need a fallback.
pub async fn verify_onchain(
    rpc_url: Option<&str>,
    contract: &str,
    wallet: &str,
    registration_tx: Option<&str>,
) -> OnChainReport {
    let url = rpc_url.map(str::trim).filter(|s| !s.is_empty());
    let Some(url) = url else {
        return OnChainReport {
            configured: false,
            checks: vec![skipped(
                "onchain",
                "BSC_RPC_URL not set — on-chain verification skipped (offline demo stays green)",
            )],
        };
    };

    let client = match BscRpcClient::new(url) {
        Ok(client) => client,
        Err(err) => {
            return OnChainReport {
                configured: true,
                checks: vec![fail(
                    "onchain_rpc_client",
                    format!("could not build RPC client: {err}"),
                )],
            };
        }
    };

    let mut checks = Vec::with_capacity(3);

    match client.chain_id().await {
        Ok(BSC_MAINNET_CHAIN_ID) => checks.push(pass(
            "onchain_chain_id",
            format!("RPC reports chainId {BSC_MAINNET_CHAIN_ID} (BSC mainnet)"),
        )),
        Ok(other) => checks.push(fail(
            "onchain_chain_id",
            format!("RPC chainId {other} != expected {BSC_MAINNET_CHAIN_ID} (not BSC mainnet)"),
        )),
        Err(err) => checks.push(fail("onchain_chain_id", format!("eth_chainId failed: {err}"))),
    }

    match client.get_code(contract).await {
        Ok(code) if has_code(&code) => checks.push(pass(
            "onchain_contract_code",
            format!(
                "competition contract {contract} has deployed bytecode ({} bytes)",
                byte_len(&code)
            ),
        )),
        Ok(_) => checks.push(fail(
            "onchain_contract_code",
            format!("no bytecode at {contract} — contract not deployed on this chain"),
        )),
        Err(err) => checks.push(fail(
            "onchain_contract_code",
            format!("eth_getCode failed: {err}"),
        )),
    }

    match registration_tx.map(str::trim).filter(|s| !s.is_empty()) {
        Some(tx) => match client.get_transaction_receipt(tx).await {
            Ok(Some(receipt)) => checks.push(receipt_check(&receipt, contract, wallet)),
            Ok(None) => checks.push(fail(
                "onchain_registration_receipt",
                format!("no receipt for {tx} — registration tx not mined on this chain"),
            )),
            Err(err) => checks.push(fail(
                "onchain_registration_receipt",
                format!("eth_getTransactionReceipt failed: {err}"),
            )),
        },
        None => checks.push(skipped(
            "onchain_registration_receipt",
            "no registration_tx anchored yet (optional in paper) — skipped",
        )),
    }

    OnChainReport {
        configured: true,
        checks,
    }
}

/// Validates a mined receipt against the expected contract.
///
/// We require `status == success` and `to == contract`. We do **not** require
/// `from == wallet`: ERC-8004 registration can be sponsored by a paymaster/relayer,
/// so a differing sender is not a failure — we only note when it does match.
fn receipt_check(receipt: &Receipt, contract: &str, wallet: &str) -> OnChainCheck {
    let status_ok = receipt.status == Some(1);
    let to_ok = receipt
        .to
        .as_deref()
        .map(|to| eq_addr(to, contract))
        .unwrap_or(false);
    let from_matches_wallet = receipt
        .from
        .as_deref()
        .map(|from| eq_addr(from, wallet))
        .unwrap_or(false);

    if status_ok && to_ok {
        let sender = if from_matches_wallet {
            format!(", from agent wallet {wallet}")
        } else {
            String::new()
        };
        let block = receipt
            .block_number
            .map(|b| format!(" in block {b}"))
            .unwrap_or_default();
        return pass(
            "onchain_registration_receipt",
            format!("registration tx mined (status=success, to={contract}{sender}){block}"),
        );
    }

    let mut reasons = Vec::new();
    if !status_ok {
        reasons.push(format!("status={:?} (expected 1=success)", receipt.status));
    }
    if !to_ok {
        reasons.push(format!(
            "to={:?} != competition contract {contract}",
            receipt.to
        ));
    }
    fail(
        "onchain_registration_receipt",
        format!("registration receipt mismatch: {}", reasons.join("; ")),
    )
}

/// Case-insensitive EVM address comparison ignoring an optional `0x` prefix.
fn eq_addr(a: &str, b: &str) -> bool {
    a.trim_start_matches("0x")
        .eq_ignore_ascii_case(b.trim_start_matches("0x"))
}

/// True when an `eth_getCode` result contains at least one non-zero byte.
fn has_code(code: &str) -> bool {
    let body = code.trim_start_matches("0x");
    !body.is_empty() && body.chars().any(|c| c != '0')
}

/// Byte length of a `0x` hex blob (two hex chars per byte).
fn byte_len(code: &str) -> usize {
    code.trim_start_matches("0x").len() / 2
}

fn pass(name: &str, detail: String) -> OnChainCheck {
    OnChainCheck {
        name: name.to_string(),
        status: CheckStatus::Pass,
        detail,
    }
}

fn fail(name: &str, detail: String) -> OnChainCheck {
    OnChainCheck {
        name: name.to_string(),
        status: CheckStatus::Fail,
        detail,
    }
}

fn skipped(name: &str, detail: &str) -> OnChainCheck {
    OnChainCheck {
        name: name.to_string(),
        status: CheckStatus::Skipped,
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTRACT: &str = "0x212c61b9b72c95d95bf29cf032f5e5635629aed5";
    const WALLET: &str = "0xA9e5C0FfEe0000000000000000000000000A1b2C3";

    #[tokio::test]
    async fn unconfigured_rpc_skips_without_failing() {
        let report = verify_onchain(None, CONTRACT, WALLET, None).await;
        assert!(!report.configured);
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].status, CheckStatus::Skipped);
        assert!(!report.checks.iter().any(|c| c.status == CheckStatus::Fail));
    }

    #[tokio::test]
    async fn empty_rpc_is_treated_as_unconfigured() {
        let report = verify_onchain(Some("   "), CONTRACT, WALLET, None).await;
        assert!(!report.configured);
        assert_eq!(report.checks[0].status, CheckStatus::Skipped);
    }

    #[test]
    fn address_equality_is_prefix_and_case_insensitive() {
        assert!(eq_addr("0xABCD", "abcd"));
        assert!(eq_addr("0xabcd", "0xABCD"));
        assert!(!eq_addr("0xabcd", "0xabce"));
    }

    #[test]
    fn code_presence_detects_real_bytecode() {
        assert!(has_code("0x60806040"));
        assert!(!has_code("0x"));
        assert!(!has_code("0x0000"));
    }

    #[test]
    fn byte_length_counts_hex_pairs() {
        assert_eq!(byte_len("0x60806040"), 4);
        assert_eq!(byte_len("0x"), 0);
    }

    #[test]
    fn receipt_passes_on_success_to_contract() {
        let receipt = Receipt {
            status: Some(1),
            to: Some(CONTRACT.to_string()),
            from: Some(WALLET.to_string()),
            block_number: Some(42),
        };
        let check = receipt_check(&receipt, CONTRACT, WALLET);
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.detail.contains("block 42"));
    }

    #[test]
    fn receipt_fails_on_revert() {
        let receipt = Receipt {
            status: Some(0),
            to: Some(CONTRACT.to_string()),
            from: None,
            block_number: Some(42),
        };
        assert_eq!(
            receipt_check(&receipt, CONTRACT, WALLET).status,
            CheckStatus::Fail
        );
    }

    #[test]
    fn receipt_fails_when_sent_to_wrong_contract() {
        let receipt = Receipt {
            status: Some(1),
            to: Some("0xdeadbeef00000000000000000000000000000000".to_string()),
            from: Some(WALLET.to_string()),
            block_number: Some(42),
        };
        assert_eq!(
            receipt_check(&receipt, CONTRACT, WALLET).status,
            CheckStatus::Fail
        );
    }
}
