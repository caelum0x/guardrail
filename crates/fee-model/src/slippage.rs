//! Price-impact and slippage estimation.
//!
//! Two effects are modeled and summed:
//!
//! 1. **Constant-product price impact.** For an AMM that holds the swap inside a
//!    constant-product pool (`x * y = k`), trading a notional `N` against
//!    available depth `L` (the side of the pool you are buying out of, valued in
//!    the same units as `N`) moves the marginal price. A standard
//!    closed-form fraction for the value lost to impact is:
//!
//!    ```text
//!    impact_fraction = N / (L + N)
//!    ```
//!
//!    This is monotonic in `N`, 0 when `N == 0`, and asymptotes to 1 as the
//!    order dwarfs the pool — exactly the behavior we want for a cost estimate.
//!
//! 2. **Linear slippage.** A configurable fixed component in basis points
//!    (1 bps = 0.01%) capturing book/queue slippage, MEV padding, or a venue's
//!    quoted spread that does not depend on pool depth.
//!
//! ```text
//! slippage_fraction = N/(L+N) + linear_bps / 10_000
//! slippage_usd      = notional_usd * slippage_fraction
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Basis-point denominator: 10_000 bps == 100%.
fn bps_denom() -> Decimal {
    Decimal::from(10_000)
}

/// Parameters for the slippage / price-impact component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlippageParams {
    /// Order notional in USD.
    pub notional_usd: Decimal,
    /// Pool liquidity / available depth in USD on the relevant side.
    pub pool_liquidity_usd: Decimal,
    /// Additional fixed linear slippage in basis points.
    pub linear_slippage_bps: Decimal,
}

impl SlippageParams {
    /// Construct slippage parameters.
    pub fn new(
        notional_usd: Decimal,
        pool_liquidity_usd: Decimal,
        linear_slippage_bps: Decimal,
    ) -> Self {
        Self {
            notional_usd,
            pool_liquidity_usd,
            linear_slippage_bps,
        }
    }

    /// Constant-product price-impact fraction: `N / (L + N)`.
    ///
    /// Returns 0 when both notional and liquidity are zero (no trade, no pool).
    pub fn price_impact_fraction(&self) -> Decimal {
        let denom = self.pool_liquidity_usd + self.notional_usd;
        if denom.is_zero() {
            return Decimal::ZERO;
        }
        self.notional_usd / denom
    }

    /// Linear slippage fraction: `linear_slippage_bps / 10_000`.
    pub fn linear_fraction(&self) -> Decimal {
        self.linear_slippage_bps / bps_denom()
    }

    /// Total slippage fraction = price impact + linear component.
    pub fn slippage_fraction(&self) -> Decimal {
        self.price_impact_fraction() + self.linear_fraction()
    }

    /// Total slippage cost in USD: `notional_usd * slippage_fraction`.
    pub fn slippage_usd(&self) -> Decimal {
        self.notional_usd * self.slippage_fraction()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn price_impact_half_pool() {
        // N = 1000, L = 1000 -> 1000/2000 = 0.5
        let s = SlippageParams::new(d("1000"), d("1000"), Decimal::ZERO);
        assert_eq!(s.price_impact_fraction(), d("0.5"));
    }

    #[test]
    fn price_impact_small_order() {
        // N = 100, L = 9900 -> 100/10000 = 0.01 (1%)
        let s = SlippageParams::new(d("100"), d("9900"), Decimal::ZERO);
        assert_eq!(s.price_impact_fraction(), d("0.01"));
    }

    #[test]
    fn linear_only() {
        // 50 bps = 0.005
        let s = SlippageParams::new(d("1000"), d("1000000"), d("50"));
        assert_eq!(s.linear_fraction(), d("0.005"));
    }

    #[test]
    fn combined_fraction_and_usd() {
        // N=100, L=9900 -> impact 0.01; +30bps=0.003 => 0.013
        // usd = 100 * 0.013 = 1.3
        let s = SlippageParams::new(d("100"), d("9900"), d("30"));
        assert_eq!(s.slippage_fraction(), d("0.013"));
        assert_eq!(s.slippage_usd(), d("1.3"));
    }

    #[test]
    fn zero_notional_zero_liquidity_is_safe() {
        let s = SlippageParams::new(Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
        assert_eq!(s.price_impact_fraction(), Decimal::ZERO);
        assert_eq!(s.slippage_usd(), Decimal::ZERO);
    }
}
