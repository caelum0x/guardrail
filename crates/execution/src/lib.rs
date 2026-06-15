//! Converts approved orders into TWAK actions.

pub mod approved_order;
pub mod error;
pub mod execution_plan;
pub mod order_intent;
pub mod post_trade;
pub mod pre_trade;
pub mod reconciliation;
pub mod retry;
pub mod router;

pub use error::ExecutionError;
pub use execution_plan::{ExecutionPlan, ExecutionResult};
pub use post_trade::{classify_receipt, SwapOutcome};
pub use pre_trade::{validate_intent, PreTradeError};
pub use reconciliation::{reconcile, Reconciliation};
pub use retry::{classify, RetryClass, RetryPolicy};
pub use router::{route, RouteDecision, Venue};

use risk_engine::{ApprovedOrder, RiskContext, RiskEngine};
use twak_client::{TwakExecutor, TxReceipt};

pub async fn execute_approved<E: TwakExecutor>(
    executor: &E,
    approved: &ApprovedOrder,
) -> Result<TxReceipt, ExecutionError> {
    executor
        .execute_swap(approved)
        .await
        .map_err(|e| ExecutionError::Twak(e.to_string()))
}

/// Execute an approved order, retrying only *transient* failures under the given
/// [`RetryPolicy`] with exponential backoff. Terminal failures (risk rejection,
/// insufficient balance, revert) are returned immediately — they must never be
/// resubmitted. On the final failed attempt the last error is returned.
pub async fn execute_with_retry<E: TwakExecutor>(
    executor: &E,
    approved: &ApprovedOrder,
    policy: &RetryPolicy,
) -> Result<TxReceipt, ExecutionError> {
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        match execute_approved(executor, approved).await {
            Ok(receipt) => return Ok(receipt),
            Err(err) => {
                let class = classify(&err.to_string());
                if !policy.should_retry(attempt, class) {
                    return Err(err);
                }
                let delay = policy.backoff(attempt);
                tracing::warn!(
                    attempt,
                    max_attempts = policy.max_attempts,
                    backoff_ms = delay.as_millis() as u64,
                    error = %err,
                    "transient execution failure; retrying after backoff"
                );
                tokio::time::sleep(delay).await;
            }
        }
    }
}

pub async fn quote_then_approve<E: TwakExecutor>(
    executor: &E,
    risk: &RiskEngine,
    intent: common::OrderIntent,
    ctx: &RiskContext,
) -> Result<ApprovedOrder, ExecutionError> {
    let pre_trade_decision = risk.pre_trade(&intent, ctx);
    let quote_intent = match &pre_trade_decision {
        risk_engine::RiskDecision::Approved => intent.clone(),
        risk_engine::RiskDecision::Clipped { new_amount_usd, .. } => {
            let mut clipped = intent.clone();
            clipped.amount_usd = *new_amount_usd;
            clipped
        }
        rejected => return Err(ExecutionError::RiskRejected(format!("{rejected:?}"))),
    };

    let quote = executor
        .quote_swap(&quote_intent)
        .await
        .map_err(|e| ExecutionError::Twak(e.to_string()))?;
    risk.approve(intent, ctx, &quote.summary)
        .map_err(|decision| ExecutionError::RiskRejected(format!("{decision:?}")))
}
