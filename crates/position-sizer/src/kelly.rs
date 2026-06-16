//! Kelly criterion position sizing.
//!
//! For a bet with win probability `p`, loss probability `q = 1 - p`, and
//! payout odds `b` (you win `b` per unit staked on a win, lose `1` per unit on
//! a loss), the Kelly-optimal fraction of capital to stake is:
//!
//! ```text
//! f* = (b * p - q) / b = edge / odds
//! ```
//!
//! where `edge = b * p - q` is the expected profit per unit staked. Full Kelly
//! maximises long-run log-growth but is volatile, so practitioners apply a
//! *fractional* Kelly multiplier (commonly `0.25`–`0.5`) and cap the result.
//!
//! This module exposes both the odds-based formulation and the
//! simple-even-money helper, then applies a fractional multiplier and a cap.

use crate::error::{ensure_in_range, ensure_positive, Result, SizingError};

/// Inputs for Kelly sizing.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct KellyInput {
    /// Probability of a winning outcome, in `[0, 1]`.
    pub win_prob: f64,
    /// Payout odds `b`: profit per unit staked on a win. Must be > 0
    /// (e.g. `1.0` for even money, `2.0` for 2:1).
    pub odds: f64,
    /// Fractional-Kelly multiplier in `[0, 1]` applied to `f*`
    /// (e.g. `0.5` = half-Kelly). Use `1.0` for full Kelly.
    pub fraction: f64,
    /// Hard cap on the final staked fraction, in `[0, 1]`. The output never
    /// exceeds this (e.g. `0.2` caps any position at 20% of capital).
    pub cap: f64,
}

/// Output of Kelly sizing.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KellyOutput {
    /// Expected edge per unit staked, `b*p - q`. May be negative.
    pub edge: f64,
    /// Raw full-Kelly fraction `f* = edge / odds` (clamped at 0 below).
    pub full_kelly: f64,
    /// `full_kelly * fraction` before the cap is applied.
    pub fractional_kelly: f64,
    /// Final fraction of capital to stake after the fractional multiplier and
    /// the cap (always in `[0, cap]`).
    pub fraction_of_capital: f64,
    /// `true` if the cap bound the final fraction.
    pub capped: bool,
}

/// Compute the Kelly stake fraction from win probability and payout odds.
///
/// Returns `f* = (b*p - q)/b`, scaled by the fractional-Kelly multiplier and
/// clamped to `[0, cap]`. A non-positive edge yields a fraction of `0` (do not
/// bet against a negative-edge proposition).
///
/// # Examples
/// ```
/// use position_sizer::kelly::{kelly_fraction, KellyInput};
/// // p=0.6, even money (b=1): f* = 2p-1 = 0.2. Half-Kelly -> 0.10.
/// let out = kelly_fraction(KellyInput {
///     win_prob: 0.6,
///     odds: 1.0,
///     fraction: 0.5,
///     cap: 1.0,
/// }).unwrap();
/// assert!((out.full_kelly - 0.2).abs() < 1e-12);
/// assert!((out.fraction_of_capital - 0.10).abs() < 1e-12);
/// ```
pub fn kelly_fraction(input: KellyInput) -> Result<KellyOutput> {
    ensure_in_range("win_prob", input.win_prob, 0.0, 1.0)?;
    ensure_positive("odds", input.odds)?;
    ensure_in_range("fraction", input.fraction, 0.0, 1.0)?;
    ensure_in_range("cap", input.cap, 0.0, 1.0)?;

    let p = input.win_prob;
    let q = 1.0 - p;
    let b = input.odds;

    // Expected edge per unit staked.
    let edge = b * p - q;
    // Full Kelly; never stake on a non-positive edge.
    let full_kelly = (edge / b).max(0.0);
    let fractional_kelly = full_kelly * input.fraction;

    let capped = fractional_kelly > input.cap;
    let fraction_of_capital = if capped {
        input.cap
    } else {
        fractional_kelly
    };

    Ok(KellyOutput {
        edge,
        full_kelly,
        fractional_kelly,
        fraction_of_capital,
        capped,
    })
}

/// Convenience constructor for an even-money (`b = 1`) Kelly stake.
///
/// For even-money bets `f* = 2p - 1`. Validates `win_prob`, then delegates to
/// [`kelly_fraction`] with `odds = 1.0`.
pub fn kelly_even_money(
    win_prob: f64,
    fraction: f64,
    cap: f64,
) -> Result<KellyOutput> {
    // Validate up front for a clear error field name before delegating.
    if !(0.0..=1.0).contains(&win_prob) || !win_prob.is_finite() {
        return Err(SizingError::OutOfRange {
            field: "win_prob",
            value: win_prob.to_string(),
            min: 0.0,
            max: 1.0,
        });
    }
    kelly_fraction(KellyInput {
        win_prob,
        odds: 1.0,
        fraction,
        cap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "expected {b}, got {a}");
    }

    #[test]
    fn known_value_even_money_full_kelly() {
        // p=0.6, b=1 -> f* = 2*0.6 - 1 = 0.2. Full Kelly, no cap.
        let out = kelly_fraction(KellyInput {
            win_prob: 0.6,
            odds: 1.0,
            fraction: 1.0,
            cap: 1.0,
        })
        .unwrap();
        approx(out.edge, 0.2);
        approx(out.full_kelly, 0.2);
        approx(out.fraction_of_capital, 0.2);
        assert!(!out.capped);
    }

    #[test]
    fn known_value_two_to_one_odds() {
        // p=0.5, b=2 -> edge = 2*0.5 - 0.5 = 0.5; f* = 0.5/2 = 0.25.
        let out = kelly_fraction(KellyInput {
            win_prob: 0.5,
            odds: 2.0,
            fraction: 1.0,
            cap: 1.0,
        })
        .unwrap();
        approx(out.edge, 0.5);
        approx(out.full_kelly, 0.25);
        approx(out.fraction_of_capital, 0.25);
    }

    #[test]
    fn half_kelly_multiplier() {
        // f* = 0.2, half-Kelly -> 0.1.
        let out = kelly_even_money(0.6, 0.5, 1.0).unwrap();
        approx(out.full_kelly, 0.2);
        approx(out.fractional_kelly, 0.1);
        approx(out.fraction_of_capital, 0.1);
        assert!(!out.capped);
    }

    #[test]
    fn cap_binds() {
        // f* = 0.2, full Kelly, but cap = 0.05.
        let out = kelly_even_money(0.6, 1.0, 0.05).unwrap();
        approx(out.full_kelly, 0.2);
        approx(out.fraction_of_capital, 0.05);
        assert!(out.capped);
    }

    #[test]
    fn negative_edge_yields_zero() {
        // p=0.4, even money -> edge = -0.2, no bet.
        let out = kelly_even_money(0.4, 1.0, 1.0).unwrap();
        approx(out.edge, -0.2);
        approx(out.full_kelly, 0.0);
        approx(out.fraction_of_capital, 0.0);
        assert!(!out.capped);
    }

    #[test]
    fn fair_coin_even_money_is_zero() {
        // p=0.5, b=1 -> edge = 0, full_kelly = 0.
        let out = kelly_even_money(0.5, 1.0, 1.0).unwrap();
        approx(out.edge, 0.0);
        approx(out.fraction_of_capital, 0.0);
    }

    #[test]
    fn rejects_probability_above_one() {
        assert!(matches!(
            kelly_even_money(1.5, 1.0, 1.0),
            Err(SizingError::OutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_zero_odds() {
        assert!(matches!(
            kelly_fraction(KellyInput {
                win_prob: 0.6,
                odds: 0.0,
                fraction: 1.0,
                cap: 1.0,
            }),
            Err(SizingError::NotPositive { .. })
        ));
    }
}
