//! Fixed-fractional position sizing.
//!
//! The fixed-fractional rule risks a constant fraction of account equity on
//! each trade. Given the per-unit risk (the distance between entry price and
//! stop-loss), the number of units to trade is:
//!
//! ```text
//! risk_capital = equity * risk_fraction
//! units        = risk_capital / risk_per_unit
//! notional     = units * entry_price
//! ```
//!
//! This is the classic "fixed fractional" / "percent risk" model used in
//! position sizing literature (e.g. Ralph Vince, Van Tharp).

use crate::error::{ensure_in_range, ensure_positive, Result};

/// Inputs for fixed-fractional sizing.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct FixedFractionalInput {
    /// Total account equity (capital available), in account currency.
    pub equity: f64,
    /// Fraction of equity to risk on this trade, in `[0, 1]` (e.g. `0.02` = 2%).
    pub risk_fraction: f64,
    /// Entry price per unit of the asset. Must be > 0.
    pub entry_price: f64,
    /// Risk per unit: `|entry_price - stop_price|`. Must be > 0.
    pub risk_per_unit: f64,
}

/// Output of fixed-fractional sizing.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FixedFractionalOutput {
    /// Amount of equity risked if the stop is hit (`equity * risk_fraction`).
    pub risk_capital: f64,
    /// Number of units to trade (`risk_capital / risk_per_unit`).
    pub units: f64,
    /// Position notional value (`units * entry_price`).
    pub notional: f64,
}

/// Compute a fixed-fractional position size.
///
/// Risks `equity * risk_fraction` and converts that into a unit count using the
/// per-unit stop distance. Returns the unit count, the notional, and the risk
/// capital. Inputs are validated; non-finite or out-of-domain values yield a
/// [`crate::error::SizingError`].
///
/// # Examples
/// ```
/// use position_sizer::fixed_fractional::{fixed_fractional, FixedFractionalInput};
/// // $100k equity, risk 1%, stop is $2 away per unit -> risk $1000 -> 500 units.
/// let out = fixed_fractional(FixedFractionalInput {
///     equity: 100_000.0,
///     risk_fraction: 0.01,
///     entry_price: 50.0,
///     risk_per_unit: 2.0,
/// }).unwrap();
/// assert_eq!(out.units, 500.0);
/// assert_eq!(out.notional, 25_000.0);
/// ```
pub fn fixed_fractional(input: FixedFractionalInput) -> Result<FixedFractionalOutput> {
    ensure_positive("equity", input.equity)?;
    ensure_in_range("risk_fraction", input.risk_fraction, 0.0, 1.0)?;
    ensure_positive("entry_price", input.entry_price)?;
    ensure_positive("risk_per_unit", input.risk_per_unit)?;

    let risk_capital = input.equity * input.risk_fraction;
    let units = risk_capital / input.risk_per_unit;
    let notional = units * input.entry_price;

    Ok(FixedFractionalOutput {
        risk_capital,
        units,
        notional,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SizingError;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "expected {b}, got {a}");
    }

    #[test]
    fn known_value_one_percent_risk() {
        // 100k * 1% = 1000 risk capital; 1000 / 2 = 500 units; 500 * 50 = 25k.
        let out = fixed_fractional(FixedFractionalInput {
            equity: 100_000.0,
            risk_fraction: 0.01,
            entry_price: 50.0,
            risk_per_unit: 2.0,
        })
        .unwrap();
        approx(out.risk_capital, 1_000.0);
        approx(out.units, 500.0);
        approx(out.notional, 25_000.0);
    }

    #[test]
    fn known_value_two_percent_risk() {
        // 50k * 2% = 1000; stop distance 0.5 -> 2000 units; entry 10 -> 20k notional.
        let out = fixed_fractional(FixedFractionalInput {
            equity: 50_000.0,
            risk_fraction: 0.02,
            entry_price: 10.0,
            risk_per_unit: 0.5,
        })
        .unwrap();
        approx(out.risk_capital, 1_000.0);
        approx(out.units, 2_000.0);
        approx(out.notional, 20_000.0);
    }

    #[test]
    fn zero_risk_fraction_gives_zero_size() {
        let out = fixed_fractional(FixedFractionalInput {
            equity: 100_000.0,
            risk_fraction: 0.0,
            entry_price: 50.0,
            risk_per_unit: 2.0,
        })
        .unwrap();
        approx(out.units, 0.0);
        approx(out.notional, 0.0);
    }

    #[test]
    fn rejects_risk_fraction_above_one() {
        assert!(matches!(
            fixed_fractional(FixedFractionalInput {
                equity: 100_000.0,
                risk_fraction: 1.5,
                entry_price: 50.0,
                risk_per_unit: 2.0,
            }),
            Err(SizingError::OutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_zero_risk_per_unit() {
        assert!(matches!(
            fixed_fractional(FixedFractionalInput {
                equity: 100_000.0,
                risk_fraction: 0.01,
                entry_price: 50.0,
                risk_per_unit: 0.0,
            }),
            Err(SizingError::NotPositive { .. })
        ));
    }

    #[test]
    fn rejects_non_finite_equity() {
        assert!(matches!(
            fixed_fractional(FixedFractionalInput {
                equity: f64::NAN,
                risk_fraction: 0.01,
                entry_price: 50.0,
                risk_per_unit: 2.0,
            }),
            Err(SizingError::NotFinite { .. })
        ));
    }
}
