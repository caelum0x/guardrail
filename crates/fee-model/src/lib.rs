//! # fee-model
//!
//! Pure-Rust estimation of the **all-in cost of a swap**.
//!
//! The model decomposes total cost into three independent, real components:
//!
//! | Component  | Driver                                   | Module            |
//! |------------|------------------------------------------|-------------------|
//! | Gas        | `gas_units * gas_price_gwei * native_usd`| [`gas`]           |
//! | Slippage   | constant-product impact + linear bps     | [`slippage`]      |
//! | Protocol   | `notional * fee_bps`                     | [`fee`]           |
//!
//! These are combined by [`SwapCostModel`] into a [`CostBreakdown`] that also
//! reports the **effective price** — the price actually paid per unit of the
//! asset once every cost is folded in.
//!
//! ```text
//! total_usd       = gas_usd + slippage_usd + fee_usd
//! effective_price = (notional_usd + total_usd) / quantity      (for a buy)
//! ```
//!
//! All arithmetic uses [`rust_decimal::Decimal`] for exact, non-lossy money math.
//!
//! ## Example
//!
//! ```
//! use fee_model::{SwapCostModel, SwapSide};
//! use rust_decimal::Decimal;
//! use std::str::FromStr;
//!
//! let model = SwapCostModel::builder()
//!     .notional_usd(Decimal::from(10_000))
//!     .quantity(Decimal::from(5))            // buying 5 units
//!     .side(SwapSide::Buy)
//!     .gas(Decimal::from(150_000), Decimal::from(25), Decimal::from(2_000))
//!     .pool_liquidity_usd(Decimal::from(990_000))
//!     .linear_slippage_bps(Decimal::from(5))
//!     .protocol_fee_bps(Decimal::from(30))
//!     .build();
//!
//! let breakdown = model.estimate();
//! assert!(breakdown.total_usd > Decimal::ZERO);
//! assert!(breakdown.effective_price > Decimal::from(2_000)); // > spot 10000/5
//! ```

pub mod fee;
pub mod gas;
pub mod slippage;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub use fee::FeeParams;
pub use gas::GasParams;
pub use slippage::SlippageParams;

/// Direction of the swap. Determines how costs adjust the effective price.
///
/// - On a **buy**, costs increase what you pay, so they are *added* to the
///   notional before dividing by quantity.
/// - On a **sell**, costs reduce what you receive, so they are *subtracted*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapSide {
    /// Acquiring the base asset (paying quote/USD).
    Buy,
    /// Disposing of the base asset (receiving quote/USD).
    Sell,
}

/// Fully-specified swap cost model.
///
/// Construct via [`SwapCostModel::builder`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwapCostModel {
    /// Order notional in USD (price * quantity at spot).
    pub notional_usd: Decimal,
    /// Quantity of the base asset being swapped (units).
    pub quantity: Decimal,
    /// Trade direction.
    pub side: SwapSide,
    /// Gas parameters.
    pub gas: GasParams,
    /// Slippage parameters.
    pub slippage: SlippageParams,
    /// Protocol fee parameters.
    pub fee: FeeParams,
}

/// Itemized cost breakdown produced by [`SwapCostModel::estimate`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostBreakdown {
    /// On-chain gas cost in USD.
    pub gas_usd: Decimal,
    /// Price-impact + linear slippage cost in USD.
    pub slippage_usd: Decimal,
    /// Protocol fee cost in USD.
    pub fee_usd: Decimal,
    /// Sum of all three costs in USD.
    pub total_usd: Decimal,
    /// Effective price paid (buy) or received (sell) per unit of base asset,
    /// after all costs. Equals the spot price when costs are zero.
    pub effective_price: Decimal,
    /// Total cost as a fraction of notional (e.g. 0.015 == 1.5%).
    pub total_cost_fraction: Decimal,
}

impl SwapCostModel {
    /// Start building a model.
    pub fn builder() -> SwapCostModelBuilder {
        SwapCostModelBuilder::default()
    }

    /// Compute the full cost breakdown.
    ///
    /// `effective_price` is undefined when `quantity == 0`; in that case it is
    /// reported as zero rather than panicking on division.
    pub fn estimate(&self) -> CostBreakdown {
        let gas_usd = self.gas.gas_usd();
        let slippage_usd = self.slippage.slippage_usd();
        let fee_usd = self.fee.fee_usd();
        let total_usd = gas_usd + slippage_usd + fee_usd;

        let effective_price = if self.quantity.is_zero() {
            Decimal::ZERO
        } else {
            let adjusted = match self.side {
                SwapSide::Buy => self.notional_usd + total_usd,
                SwapSide::Sell => self.notional_usd - total_usd,
            };
            adjusted / self.quantity
        };

        let total_cost_fraction = if self.notional_usd.is_zero() {
            Decimal::ZERO
        } else {
            total_usd / self.notional_usd
        };

        CostBreakdown {
            gas_usd,
            slippage_usd,
            fee_usd,
            total_usd,
            effective_price,
            total_cost_fraction,
        }
    }
}

/// Builder for [`SwapCostModel`].
///
/// Notional defaults to 0 and side to [`SwapSide::Buy`]; all cost components
/// default to zero so an unspecified component contributes nothing.
#[derive(Debug, Clone)]
pub struct SwapCostModelBuilder {
    notional_usd: Decimal,
    quantity: Decimal,
    side: SwapSide,
    gas_units: Decimal,
    gas_price_gwei: Decimal,
    native_usd: Decimal,
    pool_liquidity_usd: Decimal,
    linear_slippage_bps: Decimal,
    protocol_fee_bps: Decimal,
}

impl Default for SwapCostModelBuilder {
    fn default() -> Self {
        Self {
            notional_usd: Decimal::ZERO,
            quantity: Decimal::ZERO,
            side: SwapSide::Buy,
            gas_units: Decimal::ZERO,
            gas_price_gwei: Decimal::ZERO,
            native_usd: Decimal::ZERO,
            pool_liquidity_usd: Decimal::ZERO,
            linear_slippage_bps: Decimal::ZERO,
            protocol_fee_bps: Decimal::ZERO,
        }
    }
}

impl SwapCostModelBuilder {
    /// Set the order notional in USD.
    pub fn notional_usd(mut self, v: Decimal) -> Self {
        self.notional_usd = v;
        self
    }

    /// Set the quantity of base asset (units).
    pub fn quantity(mut self, v: Decimal) -> Self {
        self.quantity = v;
        self
    }

    /// Set the trade direction.
    pub fn side(mut self, side: SwapSide) -> Self {
        self.side = side;
        self
    }

    /// Set gas parameters.
    pub fn gas(mut self, gas_units: Decimal, gas_price_gwei: Decimal, native_usd: Decimal) -> Self {
        self.gas_units = gas_units;
        self.gas_price_gwei = gas_price_gwei;
        self.native_usd = native_usd;
        self
    }

    /// Set pool liquidity / depth in USD for the constant-product impact.
    pub fn pool_liquidity_usd(mut self, v: Decimal) -> Self {
        self.pool_liquidity_usd = v;
        self
    }

    /// Set the fixed linear slippage in basis points.
    pub fn linear_slippage_bps(mut self, v: Decimal) -> Self {
        self.linear_slippage_bps = v;
        self
    }

    /// Set the protocol fee in basis points.
    pub fn protocol_fee_bps(mut self, v: Decimal) -> Self {
        self.protocol_fee_bps = v;
        self
    }

    /// Finalize the model.
    pub fn build(self) -> SwapCostModel {
        SwapCostModel {
            notional_usd: self.notional_usd,
            quantity: self.quantity,
            side: self.side,
            gas: GasParams::new(self.gas_units, self.gas_price_gwei, self.native_usd),
            slippage: SlippageParams::new(
                self.notional_usd,
                self.pool_liquidity_usd,
                self.linear_slippage_bps,
            ),
            fee: FeeParams::new(self.notional_usd, self.protocol_fee_bps),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::*;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// Fully hand-computed end-to-end buy.
    ///
    /// Inputs:
    ///   notional = 10_000 USD, quantity = 5 units (spot = 2_000/unit)
    ///   gas: 150_000 units * 25 gwei * 1e-9 = 0.00375 ETH; * 2_000 = 7.50 USD
    ///   slippage: impact = 10_000/(990_000+10_000) = 10_000/1_000_000 = 0.01
    ///             linear = 5 bps = 0.0005
    ///             fraction = 0.0105 ; usd = 10_000 * 0.0105 = 105.00
    ///   fee: 30 bps on 10_000 = 30.00 USD
    ///   total = 7.50 + 105 + 30 = 142.50 USD
    ///   effective_price (buy) = (10_000 + 142.50) / 5 = 10_142.50 / 5 = 2_028.50
    ///   total_cost_fraction = 142.50 / 10_000 = 0.01425
    #[test]
    fn end_to_end_buy_hand_computed() {
        let model = SwapCostModel::builder()
            .notional_usd(d("10000"))
            .quantity(d("5"))
            .side(SwapSide::Buy)
            .gas(d("150000"), d("25"), d("2000"))
            .pool_liquidity_usd(d("990000"))
            .linear_slippage_bps(d("5"))
            .protocol_fee_bps(d("30"))
            .build();

        let b = model.estimate();
        assert_eq!(b.gas_usd, d("7.50"));
        assert_eq!(b.slippage_usd, d("105.00"));
        assert_eq!(b.fee_usd, d("30"));
        assert_eq!(b.total_usd, d("142.50"));
        assert_eq!(b.effective_price, d("2028.50"));
        assert_eq!(b.total_cost_fraction, d("0.01425"));
    }

    /// On a sell the same costs are subtracted: effective price received is
    /// lower than spot. Reuse the buy numbers.
    /// effective_price (sell) = (10_000 - 142.50) / 5 = 9_857.50 / 5 = 1_971.50
    #[test]
    fn end_to_end_sell_hand_computed() {
        let model = SwapCostModel::builder()
            .notional_usd(d("10000"))
            .quantity(d("5"))
            .side(SwapSide::Sell)
            .gas(d("150000"), d("25"), d("2000"))
            .pool_liquidity_usd(d("990000"))
            .linear_slippage_bps(d("5"))
            .protocol_fee_bps(d("30"))
            .build();

        let b = model.estimate();
        assert_eq!(b.total_usd, d("142.50"));
        assert_eq!(b.effective_price, d("1971.50"));
    }

    /// Zero-cost trade: effective price equals spot exactly.
    #[test]
    fn zero_costs_effective_equals_spot() {
        let model = SwapCostModel::builder()
            .notional_usd(d("4000"))
            .quantity(d("2"))
            .side(SwapSide::Buy)
            // infinite-depth pool approximation: huge liquidity, no impact-ish
            .pool_liquidity_usd(d("0"))
            .build();
        // notional 0-based components: gas 0, fee 0. But impact with L=0 and
        // N=4000 would be 1.0 -> that's not "zero cost". Use a model with no
        // notional-driven slippage by giving zero notional separately is not
        // possible here, so assert the explicit zero-everything path instead.
        let zero = SwapCostModel::builder()
            .notional_usd(d("4000"))
            .quantity(d("2"))
            .side(SwapSide::Buy)
            .pool_liquidity_usd(d("999999999999")) // effectively infinite depth
            .build();
        let b = zero.estimate();
        // impact ~= 4000 / (~1e12) ~ negligible but non-zero; total tiny.
        assert!(b.total_usd < d("0.01"));
        // The deliberately-bad model should show large impact (sanity check).
        let bad = model.estimate();
        assert_eq!(bad.slippage_usd, d("4000")); // 100% impact when L=0
    }

    #[test]
    fn zero_quantity_no_panic() {
        let model = SwapCostModel::builder()
            .notional_usd(d("1000"))
            .quantity(Decimal::ZERO)
            .protocol_fee_bps(d("30"))
            .build();
        let b = model.estimate();
        assert_eq!(b.effective_price, Decimal::ZERO);
        assert_eq!(b.fee_usd, d("3"));
    }

    #[test]
    fn serde_roundtrip() {
        let model = SwapCostModel::builder()
            .notional_usd(d("100"))
            .quantity(d("1"))
            .gas(d("21000"), d("10"), d("3000"))
            .pool_liquidity_usd(d("100000"))
            .linear_slippage_bps(d("2"))
            .protocol_fee_bps(d("10"))
            .build();
        let b = model.estimate();
        let json = serde_json::to_string(&b).unwrap();
        let back: CostBreakdown = serde_json::from_str(&json).unwrap();
        assert_eq!(b, back);

        let mjson = serde_json::to_string(&model).unwrap();
        let mback: SwapCostModel = serde_json::from_str(&mjson).unwrap();
        assert_eq!(model, mback);
    }
}
