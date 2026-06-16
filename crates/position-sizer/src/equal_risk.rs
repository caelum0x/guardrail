//! Equal-risk-contribution (ERC) / risk-parity weights from volatilities.
//!
//! In a risk-parity portfolio each asset contributes equally to total
//! portfolio risk. The marginal contribution of asset `i` to portfolio
//! volatility is `w_i * (Σ w)_i / σ_p`. Setting all contributions equal in
//! general requires an iterative solve, but when the covariance matrix is
//! diagonal (assets treated as uncorrelated, or under an equal-correlation
//! assumption) the solution has the well-known closed form:
//!
//! ```text
//! w_i = (1 / σ_i) / Σ_j (1 / σ_j)
//! ```
//!
//! i.e. weights are proportional to inverse volatility and normalised to sum
//! to one. Each asset then contributes `1/N` of the (uncorrelated) portfolio
//! variance. This is the standard "inverse-volatility" risk-parity allocation.

use crate::error::{ensure_positive, Result, SizingError};

/// One asset's allocation in the ERC portfolio.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AssetWeight {
    /// Identifier supplied by the caller (e.g. ticker / symbol).
    pub asset: String,
    /// Volatility used for the allocation (echoed back for traceability).
    pub vol: f64,
    /// Portfolio weight in `[0, 1]`; weights sum to `1`.
    pub weight: f64,
    /// Risk contribution under the diagonal-covariance model, as a fraction of
    /// total portfolio variance. For inverse-vol weights this is `1/N` for
    /// every asset.
    pub risk_contribution: f64,
}

/// Compute inverse-volatility (equal-risk-contribution) weights.
///
/// Accepts `(asset_id, volatility)` pairs and returns normalised weights that
/// equalise each asset's risk contribution under a diagonal covariance model.
/// All volatilities must be finite and strictly positive, and at least one
/// asset must be supplied.
///
/// # Examples
/// ```
/// use position_sizer::equal_risk::equal_risk_contribution;
/// // Two assets, 10% and 20% vol. Inverse-vol: 1/0.1=10, 1/0.2=5; sum=15.
/// // Weights: 10/15 = 2/3, 5/15 = 1/3.
/// let w = equal_risk_contribution(&[
///     ("A".to_string(), 0.10),
///     ("B".to_string(), 0.20),
/// ]).unwrap();
/// assert!((w[0].weight - 2.0 / 3.0).abs() < 1e-12);
/// assert!((w[1].weight - 1.0 / 3.0).abs() < 1e-12);
/// // Equal risk contribution: each is 1/2.
/// assert!((w[0].risk_contribution - 0.5).abs() < 1e-12);
/// ```
pub fn equal_risk_contribution(assets: &[(String, f64)]) -> Result<Vec<AssetWeight>> {
    if assets.is_empty() {
        return Err(SizingError::EmptyInput { field: "assets" });
    }

    // Validate and accumulate the inverse-vol normaliser.
    let mut inv_vol = Vec::with_capacity(assets.len());
    let mut inv_sum = 0.0_f64;
    for (id, vol) in assets {
        ensure_positive("vol", *vol)?;
        let iv = 1.0 / *vol;
        inv_sum += iv;
        inv_vol.push((id.clone(), *vol, iv));
    }

    // inv_sum > 0 is guaranteed since every vol is finite and positive.
    let n = assets.len() as f64;
    let weights = inv_vol
        .into_iter()
        .map(|(asset, vol, iv)| AssetWeight {
            asset,
            vol,
            weight: iv / inv_sum,
            // Under the diagonal model inverse-vol weights split variance evenly.
            risk_contribution: 1.0 / n,
        })
        .collect();

    Ok(weights)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-12, "expected {b}, got {a}");
    }

    fn assets(pairs: &[(&str, f64)]) -> Vec<(String, f64)> {
        pairs.iter().map(|(s, v)| (s.to_string(), *v)).collect()
    }

    #[test]
    fn known_value_two_assets() {
        let w = equal_risk_contribution(&assets(&[("A", 0.10), ("B", 0.20)])).unwrap();
        approx(w[0].weight, 2.0 / 3.0);
        approx(w[1].weight, 1.0 / 3.0);
        approx(w[0].weight + w[1].weight, 1.0);
    }

    #[test]
    fn equal_vols_give_equal_weights() {
        let w =
            equal_risk_contribution(&assets(&[("A", 0.15), ("B", 0.15), ("C", 0.15)]))
                .unwrap();
        for a in &w {
            approx(a.weight, 1.0 / 3.0);
            approx(a.risk_contribution, 1.0 / 3.0);
        }
    }

    #[test]
    fn lower_vol_gets_higher_weight() {
        let w = equal_risk_contribution(&assets(&[("Low", 0.05), ("High", 0.50)]))
            .unwrap();
        // 1/0.05 = 20, 1/0.5 = 2, sum = 22.
        approx(w[0].weight, 20.0 / 22.0);
        approx(w[1].weight, 2.0 / 22.0);
        assert!(w[0].weight > w[1].weight);
    }

    #[test]
    fn weights_sum_to_one_for_many_assets() {
        let w = equal_risk_contribution(&assets(&[
            ("A", 0.08),
            ("B", 0.12),
            ("C", 0.25),
            ("D", 0.40),
        ]))
        .unwrap();
        let total: f64 = w.iter().map(|a| a.weight).sum();
        approx(total, 1.0);
    }

    #[test]
    fn risk_contributions_are_equal_and_sum_to_one() {
        // Verify the actual equal-risk property under the diagonal model:
        // RC_i = w_i^2 * sigma_i^2 / sum_j(w_j^2 sigma_j^2) should equal 1/N.
        let pairs = assets(&[("A", 0.10), ("B", 0.20), ("C", 0.30)]);
        let w = equal_risk_contribution(&pairs).unwrap();
        let port_var: f64 = w.iter().map(|a| (a.weight * a.vol).powi(2)).sum();
        let n = w.len() as f64;
        let mut sum_rc = 0.0;
        for a in &w {
            let rc = (a.weight * a.vol).powi(2) / port_var;
            approx(rc, 1.0 / n);
            sum_rc += rc;
        }
        approx(sum_rc, 1.0);
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            equal_risk_contribution(&[]),
            Err(SizingError::EmptyInput { .. })
        ));
    }

    #[test]
    fn rejects_zero_vol() {
        assert!(matches!(
            equal_risk_contribution(&assets(&[("A", 0.0)])),
            Err(SizingError::NotPositive { .. })
        ));
    }
}
