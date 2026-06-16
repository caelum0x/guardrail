//! Gas cost estimation.
//!
//! On EVM chains the on-chain transaction fee is:
//!
//! ```text
//! gas_fee_native = gas_units * gas_price (in native token)
//! ```
//!
//! Gas prices are conventionally quoted in gwei (1 gwei = 1e-9 native token).
//! Converting to USD requires the spot price of the native token:
//!
//! ```text
//! gas_usd = gas_units * gas_price_gwei * 1e-9 * native_usd
//! ```

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 1 gwei expressed in whole native tokens: 1e-9.
/// `Decimal::new(1, 9)` == 0.000000001.
fn gwei_scale() -> Decimal {
    Decimal::new(1, 9)
}

/// Parameters describing the on-chain gas component of a swap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GasParams {
    /// Estimated gas units the swap transaction will consume.
    pub gas_units: Decimal,
    /// Gas price in gwei (1 gwei = 1e-9 native token).
    pub gas_price_gwei: Decimal,
    /// Spot price of the native token (e.g. ETH) in USD.
    pub native_usd: Decimal,
}

impl GasParams {
    /// Construct gas parameters.
    pub fn new(gas_units: Decimal, gas_price_gwei: Decimal, native_usd: Decimal) -> Self {
        Self {
            gas_units,
            gas_price_gwei,
            native_usd,
        }
    }

    /// Fee in native token units (e.g. ETH).
    ///
    /// `gas_units * gas_price_gwei * 1e-9`
    pub fn gas_native(&self) -> Decimal {
        self.gas_units * self.gas_price_gwei * gwei_scale()
    }

    /// Fee converted to USD: `gas_native * native_usd`.
    pub fn gas_usd(&self) -> Decimal {
        self.gas_native() * self.native_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    #[test]
    fn gas_native_simple() {
        // 100_000 gas @ 20 gwei = 100000 * 20 * 1e-9 = 0.002 ETH
        let g = GasParams::new(dec(100_000), dec(20), dec(2000));
        assert_eq!(g.gas_native(), Decimal::from_str("0.002").unwrap());
    }

    #[test]
    fn gas_usd_hand_computed() {
        // 0.002 ETH * 2000 USD/ETH = 4.00 USD
        let g = GasParams::new(dec(100_000), dec(20), dec(2000));
        assert_eq!(g.gas_usd(), Decimal::from_str("4.0").unwrap());
    }

    #[test]
    fn zero_gas_price_is_free() {
        let g = GasParams::new(dec(21_000), dec(0), dec(3000));
        assert_eq!(g.gas_usd(), Decimal::ZERO);
    }

    fn dec(n: i64) -> Decimal {
        Decimal::from(n)
    }
}
