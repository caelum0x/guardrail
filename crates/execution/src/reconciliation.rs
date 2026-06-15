//! Post-swap reconciliation: did we get what the quote promised?
//!
//! After a confirmed swap, the realized output is compared against the quoted
//! `expected_out_usd` to compute the *realized slippage* in basis points. If it
//! exceeds the policy tolerance the swap is flagged so the loop can log/alert —
//! this is how silent execution degradation (MEV, stale quotes) gets caught.

use common::{Decimal, QuoteSummary};

/// Result of reconciling a confirmed swap's realized output against its quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciliation {
    pub expected_out_usd: Decimal,
    pub realized_out_usd: Decimal,
    /// Positive = received less than quoted (adverse); negative = better fill.
    pub realized_slippage_bps: Decimal,
    /// True when adverse slippage exceeded the tolerance.
    pub breached_tolerance: bool,
}

const BPS_SCALE: i64 = 10_000;

/// Reconcile a confirmed swap.
///
/// `tolerance_bps` is the maximum adverse slippage (in basis points) considered
/// acceptable. Realized slippage is `(expected - realized) / expected` in bps; a
/// negative value means a better-than-quoted fill and never breaches.
pub fn reconcile(
    quote: &QuoteSummary,
    realized_out_usd: Decimal,
    tolerance_bps: Decimal,
) -> Reconciliation {
    let expected = quote.expected_out_usd;
    let realized_slippage_bps = if expected > Decimal::ZERO {
        (expected - realized_out_usd) / expected * Decimal::from(BPS_SCALE)
    } else {
        Decimal::ZERO
    };

    Reconciliation {
        expected_out_usd: expected,
        realized_out_usd,
        realized_slippage_bps,
        breached_tolerance: realized_slippage_bps > tolerance_bps,
    }
}

/// Whether reconciliation should run at all (retained for callers that gate on
/// a simple flag).
pub fn reconciliation_required() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(expected: i64) -> QuoteSummary {
        QuoteSummary {
            expected_out_usd: Decimal::from(expected),
            price_impact_pct: Decimal::ZERO,
            slippage_pct: Decimal::ZERO,
            liquidity_usd: Decimal::from(1_000_000),
        }
    }

    #[test]
    fn adverse_fill_breaches_tolerance() {
        // expected 1000, realized 990 -> 100 bps adverse, tolerance 50 -> breach.
        let r = reconcile(&quote(1000), Decimal::from(990), Decimal::from(50));
        assert_eq!(r.realized_slippage_bps, Decimal::from(100));
        assert!(r.breached_tolerance);
    }

    #[test]
    fn better_fill_never_breaches() {
        let r = reconcile(&quote(1000), Decimal::from(1010), Decimal::from(50));
        assert_eq!(r.realized_slippage_bps, Decimal::from(-100));
        assert!(!r.breached_tolerance);
    }

    #[test]
    fn within_tolerance_passes() {
        let r = reconcile(&quote(1000), Decimal::from(997), Decimal::from(50));
        assert_eq!(r.realized_slippage_bps, Decimal::from(30));
        assert!(!r.breached_tolerance);
    }
}
