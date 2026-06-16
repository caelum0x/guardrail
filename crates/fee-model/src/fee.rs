//! Protocol fee estimation.
//!
//! Most AMMs/aggregators charge a fee as a fraction of the swapped notional,
//! quoted in basis points (e.g. Uniswap v3 0.30% tier == 30 bps):
//!
//! ```text
//! fee_usd = notional_usd * protocol_fee_bps / 10_000
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Basis-point denominator: 10_000 bps == 100%.
fn bps_denom() -> Decimal {
    Decimal::from(10_000)
}

/// Parameters for the protocol fee component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeParams {
    /// Order notional in USD.
    pub notional_usd: Decimal,
    /// Protocol fee in basis points (e.g. 30 == 0.30%).
    pub protocol_fee_bps: Decimal,
}

impl FeeParams {
    /// Construct fee parameters.
    pub fn new(notional_usd: Decimal, protocol_fee_bps: Decimal) -> Self {
        Self {
            notional_usd,
            protocol_fee_bps,
        }
    }

    /// Fee fraction: `protocol_fee_bps / 10_000`.
    pub fn fee_fraction(&self) -> Decimal {
        self.protocol_fee_bps / bps_denom()
    }

    /// Fee cost in USD: `notional_usd * fee_fraction`.
    pub fn fee_usd(&self) -> Decimal {
        self.notional_usd * self.fee_fraction()
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
    fn uniswap_030_tier() {
        // 30 bps on 10_000 USD = 30 USD
        let f = FeeParams::new(d("10000"), d("30"));
        assert_eq!(f.fee_fraction(), d("0.003"));
        assert_eq!(f.fee_usd(), d("30"));
    }

    #[test]
    fn five_bps_tier() {
        // 5 bps on 50_000 = 25 USD
        let f = FeeParams::new(d("50000"), d("5"));
        assert_eq!(f.fee_usd(), d("25"));
    }

    #[test]
    fn zero_fee() {
        let f = FeeParams::new(d("1000"), Decimal::ZERO);
        assert_eq!(f.fee_usd(), Decimal::ZERO);
    }
}
