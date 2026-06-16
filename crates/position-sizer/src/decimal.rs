//! Exact decimal sizing helpers using [`rust_decimal`].
//!
//! Floating-point sizing (the other modules) is fine for ratios and leverage,
//! but converting a target notional into an order quantity that must round to a
//! venue lot/tick should be done with exact decimal arithmetic to avoid
//! accumulating binary-float error. This module provides a fixed-fractional
//! sizing routine and a lot-rounding helper built on `Decimal`.

use crate::error::{Result, SizingError};
use rust_decimal::Decimal;

/// Rounding direction for converting a raw quantity to a tradable lot size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LotRounding {
    /// Round down to the nearest lot (never over-allocate). Default for risk.
    Floor,
    /// Round to the nearest lot.
    Nearest,
}

/// Exact fixed-fractional sizing in decimal arithmetic.
///
/// Computes `units = (equity * risk_fraction) / risk_per_unit` using
/// [`Decimal`], then rounds to a whole multiple of `lot_size` per `rounding`.
/// Returns the tradable unit count.
///
/// All inputs must be positive (`equity`, `risk_per_unit`, `lot_size`) or in
/// range (`risk_fraction` in `[0, 1]`).
pub fn fixed_fractional_units(
    equity: Decimal,
    risk_fraction: Decimal,
    risk_per_unit: Decimal,
    lot_size: Decimal,
    rounding: LotRounding,
) -> Result<Decimal> {
    if equity <= Decimal::ZERO {
        return Err(SizingError::NotPositive {
            field: "equity",
            value: equity.to_string(),
        });
    }
    if risk_fraction < Decimal::ZERO || risk_fraction > Decimal::ONE {
        return Err(SizingError::OutOfRange {
            field: "risk_fraction",
            value: risk_fraction.to_string(),
            min: 0.0,
            max: 1.0,
        });
    }
    if risk_per_unit <= Decimal::ZERO {
        return Err(SizingError::NotPositive {
            field: "risk_per_unit",
            value: risk_per_unit.to_string(),
        });
    }
    if lot_size <= Decimal::ZERO {
        return Err(SizingError::NotPositive {
            field: "lot_size",
            value: lot_size.to_string(),
        });
    }

    let risk_capital = equity * risk_fraction;
    let raw_units = risk_capital / risk_per_unit;

    // Number of whole lots, rounded per the requested mode.
    let lots = raw_units / lot_size;
    let rounded_lots = match rounding {
        LotRounding::Floor => lots.floor(),
        LotRounding::Nearest => lots.round(),
    };

    Ok(rounded_lots * lot_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros_free::dec;

    // Tiny local `dec!`-style constructor to avoid an extra dependency.
    mod rust_decimal_macros_free {
        macro_rules! dec {
            ($v:expr) => {{
                use core::str::FromStr;
                rust_decimal::Decimal::from_str(stringify!($v)).unwrap()
            }};
        }
        pub(crate) use dec;
    }

    #[test]
    fn floor_rounds_down_to_lot() {
        // equity 100000, risk 1% -> 1000 risk capital; risk/unit 3 -> 333.33 units.
        // lot 1 -> floor to 333.
        let units = fixed_fractional_units(
            dec!(100000),
            dec!(0.01),
            dec!(3),
            dec!(1),
            LotRounding::Floor,
        )
        .unwrap();
        assert_eq!(units, dec!(333));
    }

    #[test]
    fn nearest_rounds_to_lot() {
        let units = fixed_fractional_units(
            dec!(100000),
            dec!(0.01),
            dec!(3),
            dec!(1),
            LotRounding::Nearest,
        )
        .unwrap();
        // 333.33... rounds to 333.
        assert_eq!(units, dec!(333));
    }

    #[test]
    fn respects_lot_size_of_ten() {
        // 1000 / 2 = 500 units exactly; lot 10 -> 500.
        let units = fixed_fractional_units(
            dec!(100000),
            dec!(0.01),
            dec!(2),
            dec!(10),
            LotRounding::Floor,
        )
        .unwrap();
        assert_eq!(units, dec!(500));
    }

    #[test]
    fn floor_with_large_lot_truncates() {
        // 500 raw units, lot 100 -> 5 lots -> 500. Now make it 540 raw.
        // equity 108000 * 1% = 1080; /2 = 540; lot 100 -> floor 5 lots -> 500.
        let units = fixed_fractional_units(
            dec!(108000),
            dec!(0.01),
            dec!(2),
            dec!(100),
            LotRounding::Floor,
        )
        .unwrap();
        assert_eq!(units, dec!(500));
    }

    #[test]
    fn exact_decimal_no_float_drift() {
        // 0.1 + 0.2 style drift would break a float impl; decimal is exact.
        // equity 30, risk 100%, risk/unit 0.1 -> 300 units exactly.
        let units = fixed_fractional_units(
            dec!(30),
            dec!(1),
            dec!(0.1),
            dec!(1),
            LotRounding::Floor,
        )
        .unwrap();
        assert_eq!(units, dec!(300));
    }

    #[test]
    fn rejects_zero_lot_size() {
        assert!(matches!(
            fixed_fractional_units(
                dec!(100000),
                dec!(0.01),
                dec!(2),
                dec!(0),
                LotRounding::Floor,
            ),
            Err(SizingError::NotPositive { .. })
        ));
    }
}
