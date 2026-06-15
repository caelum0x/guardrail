//! Retry policy for transient execution failures.
//!
//! Swap submission can fail for transient reasons (RPC hiccup, nonce race,
//! mempool congestion) or terminal ones (risk rejection, insufficient balance,
//! reverted transaction). This module classifies failures and computes a
//! bounded exponential backoff so the trading loop retries the right errors the
//! right number of times — and never retries a terminal one.

use std::time::Duration;

/// How an execution error should be handled on the next attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// Worth retrying after a backoff (transient infrastructure failure).
    Transient,
    /// Must not be retried (the order itself is invalid or was rejected).
    Terminal,
}

/// A bounded exponential-backoff retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the first), always `>= 1`.
    pub max_attempts: u32,
    /// Delay before the first retry.
    pub base_delay: Duration,
    /// Backoff is capped at this delay regardless of attempt count.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(5),
        }
    }
}

impl RetryPolicy {
    /// Build a policy, clamping `max_attempts` to at least 1.
    pub fn new(max_attempts: u32, base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            base_delay,
            max_delay,
        }
    }

    /// Whether another attempt should be made, given the 1-based attempt that
    /// just failed and how that failure was classified. Terminal failures are
    /// never retried.
    pub fn should_retry(&self, attempt: u32, class: RetryClass) -> bool {
        class == RetryClass::Transient && attempt < self.max_attempts
    }

    /// Deterministic exponential backoff before the next retry:
    /// `base * 2^(attempt-1)`, capped at `max_delay`. `attempt` is the 1-based
    /// attempt that just failed.
    pub fn backoff(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.base_delay.min(self.max_delay);
        }
        // Saturating shift so a large attempt count can't overflow.
        let factor = 1u64.checked_shl(attempt - 1).unwrap_or(u64::MAX);
        let scaled = self
            .base_delay
            .checked_mul(factor.min(u32::MAX as u64) as u32)
            .unwrap_or(self.max_delay);
        scaled.min(self.max_delay)
    }
}

/// Classify an execution error message into transient vs terminal. The matching
/// is conservative: anything that looks like a risk rejection, balance issue, or
/// revert is terminal; recognized infrastructure errors are transient; unknown
/// errors default to terminal (fail safe — never resubmit a money-moving order
/// we do not understand).
pub fn classify(error_message: &str) -> RetryClass {
    let m = error_message.to_ascii_lowercase();

    const TERMINAL: [&str; 7] = [
        "risk rejected",
        "insufficient",
        "reverted",
        "invalid",
        "unauthorized",
        "slippage exceeded",
        "rejected",
    ];
    if TERMINAL.iter().any(|k| m.contains(k)) {
        return RetryClass::Terminal;
    }

    const TRANSIENT: [&str; 6] = [
        "timeout",
        "timed out",
        "connection",
        "temporarily",
        "rate limit",
        "nonce",
    ];
    if TRANSIENT.iter().any(|k| m.contains(k)) {
        return RetryClass::Transient;
    }

    RetryClass::Terminal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_errors_are_not_retried() {
        let p = RetryPolicy::default();
        assert_eq!(classify("risk rejected order"), RetryClass::Terminal);
        assert_eq!(classify("insufficient balance"), RetryClass::Terminal);
        assert!(!p.should_retry(1, RetryClass::Terminal));
    }

    #[test]
    fn transient_errors_retry_until_limit() {
        let p = RetryPolicy::new(3, Duration::from_millis(100), Duration::from_secs(2));
        assert_eq!(classify("connection timeout"), RetryClass::Transient);
        assert!(p.should_retry(1, RetryClass::Transient));
        assert!(p.should_retry(2, RetryClass::Transient));
        assert!(!p.should_retry(3, RetryClass::Transient));
    }

    #[test]
    fn unknown_errors_fail_safe_terminal() {
        assert_eq!(classify("something weird happened"), RetryClass::Terminal);
    }

    #[test]
    fn backoff_grows_then_caps() {
        let p = RetryPolicy::new(10, Duration::from_millis(100), Duration::from_millis(400));
        assert_eq!(p.backoff(1), Duration::from_millis(100));
        assert_eq!(p.backoff(2), Duration::from_millis(200));
        assert_eq!(p.backoff(3), Duration::from_millis(400));
        assert_eq!(p.backoff(4), Duration::from_millis(400)); // capped
        assert_eq!(p.backoff(40), Duration::from_millis(400)); // no overflow
    }
}
