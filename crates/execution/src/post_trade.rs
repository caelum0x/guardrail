//! Post-trade processing of a swap receipt.
//!
//! After the executor returns a [`TxReceipt`], the loop needs to know whether
//! the swap landed, failed, or is still pending, and whether portfolio
//! reconciliation should run. This module normalizes the free-form `status`
//! string into a typed outcome.

use twak_client::TxReceipt;

/// Default policy: reconcile the portfolio after every confirmed swap.
pub const RECONCILE_AFTER_SWAP: bool = true;

/// Normalized outcome of a submitted swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapOutcome {
    /// Mined/settled successfully.
    Confirmed,
    /// Accepted by the mempool but not yet mined.
    Pending,
    /// Reverted or rejected on-chain / by the executor.
    Failed,
    /// Status string not recognized.
    Unknown,
}

impl SwapOutcome {
    /// Reconciliation is meaningful only once funds have actually moved.
    pub fn should_reconcile(self) -> bool {
        RECONCILE_AFTER_SWAP && matches!(self, SwapOutcome::Confirmed)
    }

    /// Whether the loop should keep waiting on this receipt.
    pub fn is_pending(self) -> bool {
        matches!(self, SwapOutcome::Pending)
    }
}

/// Classify a receipt's status into a typed outcome.
pub fn classify_receipt(receipt: &TxReceipt) -> SwapOutcome {
    match receipt.status.to_ascii_lowercase().as_str() {
        "confirmed" | "success" | "succeeded" | "mined" | "ok" => SwapOutcome::Confirmed,
        "submitted" | "pending" | "queued" | "broadcast" => SwapOutcome::Pending,
        "failed" | "reverted" | "rejected" | "error" => SwapOutcome::Failed,
        _ => SwapOutcome::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(status: &str) -> TxReceipt {
        TxReceipt {
            tx_hash: "0xabc".into(),
            status: status.into(),
            block_number: None,
        }
    }

    #[test]
    fn classifies_statuses() {
        assert_eq!(
            classify_receipt(&receipt("confirmed")),
            SwapOutcome::Confirmed
        );
        assert_eq!(
            classify_receipt(&receipt("submitted")),
            SwapOutcome::Pending
        );
        assert_eq!(classify_receipt(&receipt("reverted")), SwapOutcome::Failed);
        assert_eq!(classify_receipt(&receipt("???")), SwapOutcome::Unknown);
    }

    #[test]
    fn only_confirmed_triggers_reconcile() {
        assert!(classify_receipt(&receipt("success")).should_reconcile());
        assert!(!classify_receipt(&receipt("pending")).should_reconcile());
        assert!(classify_receipt(&receipt("submitted")).is_pending());
    }
}
