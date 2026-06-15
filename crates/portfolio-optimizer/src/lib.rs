//! Alternative portfolio allocation methods.
//!
//! Every public function returns a `Vec<f64>` of non-negative weights aligned
//! to the input order. When a meaningful allocation exists the weights sum to
//! `budget`; when no positive signal is available (e.g. all scores or all
//! volatilities are non-positive) the function returns a zero vector rather
//! than fabricating an allocation.

use serde::{Deserialize, Serialize};

/// Sanitizes the requested budget. Non-finite or negative budgets collapse to
/// zero so callers never receive nonsensical allocations.
fn sanitize_budget(budget: f64) -> f64 {
    if budget.is_finite() && budget > 0.0 {
        budget
    } else {
        0.0
    }
}

/// Distributes `budget` evenly across `n` assets.
///
/// Returns an empty vector when `n == 0` and a zero vector when the budget is
/// non-positive.
pub fn equal_weight(n: usize, budget: f64) -> Vec<f64> {
    let budget = sanitize_budget(budget);
    if n == 0 {
        return Vec::new();
    }
    let share = budget / n as f64;
    vec![share; n]
}

/// Allocates `budget` proportionally to each asset's positive score.
///
/// Scores that are non-positive (or non-finite) contribute nothing. If no
/// score is positive the result is a zero vector aligned to the input.
pub fn score_proportional(scores: &[f64], budget: f64) -> Vec<f64> {
    let budget = sanitize_budget(budget);
    let positive: Vec<f64> = scores
        .iter()
        .map(|&s| if s.is_finite() && s > 0.0 { s } else { 0.0 })
        .collect();
    let total: f64 = positive.iter().sum();
    if total <= 0.0 {
        return vec![0.0; scores.len()];
    }
    positive.into_iter().map(|s| budget * s / total).collect()
}

/// Allocates `budget` proportionally to the inverse of each asset's volatility.
///
/// Lower-volatility assets receive larger weights. Non-positive or non-finite
/// volatilities are skipped (weight 0). If no volatility is usable the result
/// is a zero vector aligned to the input.
pub fn inverse_volatility(vols: &[f64], budget: f64) -> Vec<f64> {
    let budget = sanitize_budget(budget);
    let inv: Vec<f64> = vols
        .iter()
        .map(|&v| {
            if v.is_finite() && v > 0.0 {
                1.0 / v
            } else {
                0.0
            }
        })
        .collect();
    let total: f64 = inv.iter().sum();
    if total <= 0.0 {
        return vec![0.0; vols.len()];
    }
    inv.into_iter().map(|w| budget * w / total).collect()
}

/// Risk-parity (lite): an equal-risk-contribution approximation that reduces to
/// the inverse-volatility allocation. Shares the same skipping/normalization
/// semantics as [`inverse_volatility`].
pub fn risk_parity_lite(vols: &[f64], budget: f64) -> Vec<f64> {
    inverse_volatility(vols, budget)
}

/// Selectable allocation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocationMethod {
    EqualWeight,
    ScoreProportional,
    InverseVolatility,
    RiskParity,
}

/// Dispatches to the allocation method indicated by `method`.
///
/// `scores` is used by [`AllocationMethod::ScoreProportional`] and determines
/// the asset count for [`AllocationMethod::EqualWeight`]; `vols` is used by the
/// volatility-based methods.
pub fn allocate(method: AllocationMethod, scores: &[f64], vols: &[f64], budget: f64) -> Vec<f64> {
    match method {
        AllocationMethod::EqualWeight => equal_weight(scores.len(), budget),
        AllocationMethod::ScoreProportional => score_proportional(scores, budget),
        AllocationMethod::InverseVolatility => inverse_volatility(vols, budget),
        AllocationMethod::RiskParity => risk_parity_lite(vols, budget),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn assert_sum(weights: &[f64], expected: f64) {
        let sum: f64 = weights.iter().sum();
        assert!(
            (sum - expected).abs() < EPS,
            "sum {sum} != expected {expected}"
        );
    }

    fn assert_non_negative(weights: &[f64]) {
        assert!(weights.iter().all(|&w| w >= 0.0), "found negative weight");
    }

    #[test]
    fn equal_weight_splits_evenly() {
        let w = equal_weight(4, 1.0);
        assert_eq!(w.len(), 4);
        assert_non_negative(&w);
        assert_sum(&w, 1.0);
        for share in &w {
            assert!((share - 0.25).abs() < EPS);
        }
    }

    #[test]
    fn score_proportional_sums_to_budget() {
        let w = score_proportional(&[2.0, 1.0, 1.0], 1.0);
        assert_non_negative(&w);
        assert_sum(&w, 1.0);
        assert!(w[0] > w[1]);
    }

    #[test]
    fn score_proportional_zero_when_no_positive() {
        let w = score_proportional(&[-1.0, 0.0, -3.0], 1.0);
        assert_eq!(w, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn inverse_volatility_favors_lower_vol() {
        let w = inverse_volatility(&[0.1, 0.4], 1.0);
        assert_non_negative(&w);
        assert_sum(&w, 1.0);
        assert!(w[0] > w[1], "lower-vol asset should get more weight");
    }

    #[test]
    fn risk_parity_matches_inverse_vol() {
        let vols = [0.2, 0.3, 0.5];
        assert_eq!(risk_parity_lite(&vols, 1.0), inverse_volatility(&vols, 1.0));
    }

    #[test]
    fn all_methods_sum_to_budget_and_non_negative() {
        let scores = [1.0, 2.0, 3.0];
        let vols = [0.2, 0.3, 0.5];
        let budget = 1000.0;
        for method in [
            AllocationMethod::EqualWeight,
            AllocationMethod::ScoreProportional,
            AllocationMethod::InverseVolatility,
            AllocationMethod::RiskParity,
        ] {
            let w = allocate(method, &scores, &vols, budget);
            assert_non_negative(&w);
            assert_sum(&w, budget);
        }
    }

    #[test]
    fn non_positive_budget_yields_zero() {
        assert_eq!(equal_weight(3, 0.0), vec![0.0, 0.0, 0.0]);
        assert_eq!(equal_weight(3, -5.0), vec![0.0, 0.0, 0.0]);
    }
}
