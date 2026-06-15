use common::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_return_pct: Decimal,
    pub max_drawdown_pct: Decimal,
    pub trade_count: u64,
    pub win_rate_pct: Decimal,
    pub profit_factor: Decimal,
    pub volatility_pct: Decimal,
    pub calmar_ratio: Decimal,
}

impl BacktestMetrics {
    /// Compute metrics from a NAV equity curve and the number of booked trades.
    ///
    /// - `total_return_pct`: end-to-end NAV change.
    /// - `max_drawdown_pct`: largest peak-to-trough decline along the curve.
    /// - `win_rate_pct`: share of step-over-step NAV increases.
    /// - `profit_factor`: gross gains divided by gross losses across steps.
    /// - `volatility_pct`: standard deviation of step-over-step percent returns.
    /// - `calmar_ratio`: total return divided by max drawdown (0 when no drawdown).
    pub fn from_curve(starting_nav: Decimal, curve: &[Decimal], trades: u64) -> Self {
        if curve.is_empty() || starting_nav <= Decimal::ZERO {
            return BacktestMetrics {
                trade_count: trades,
                ..Default::default()
            };
        }
        let hundred = Decimal::from(100);
        let final_nav = *curve.last().unwrap();
        let total_return_pct = (final_nav - starting_nav) / starting_nav * hundred;

        let mut peak = starting_nav;
        let mut max_dd = Decimal::ZERO;
        let mut prev = starting_nav;
        let mut wins = 0u64;
        let mut steps = 0u64;
        let mut gains = Decimal::ZERO;
        let mut losses = Decimal::ZERO;
        let mut returns: Vec<Decimal> = Vec::with_capacity(curve.len());

        for &nav in curve {
            if nav > peak {
                peak = nav;
            }
            if peak > Decimal::ZERO {
                let dd = (peak - nav) / peak * hundred;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
            let delta = nav - prev;
            steps += 1;
            if delta > Decimal::ZERO {
                wins += 1;
                gains += delta;
            } else if delta < Decimal::ZERO {
                losses += -delta;
            }
            if prev > Decimal::ZERO {
                returns.push(delta / prev * hundred);
            }
            prev = nav;
        }

        let win_rate_pct = if steps > 0 {
            Decimal::from(wins) / Decimal::from(steps) * hundred
        } else {
            Decimal::ZERO
        };
        let profit_factor = if losses > Decimal::ZERO {
            (gains / losses).round_dp(3)
        } else {
            Decimal::ZERO
        };

        let total_return_pct = total_return_pct.round_dp(3);
        let max_drawdown_pct = max_dd.round_dp(3);

        let volatility_pct = std_dev_pct(&returns);
        let calmar_ratio = if max_drawdown_pct > Decimal::ZERO {
            (total_return_pct / max_drawdown_pct).round_dp(3)
        } else {
            Decimal::ZERO
        };

        BacktestMetrics {
            total_return_pct,
            max_drawdown_pct,
            trade_count: trades,
            win_rate_pct: win_rate_pct.round_dp(2),
            profit_factor,
            volatility_pct,
            calmar_ratio,
        }
    }
}

/// Population standard deviation of a set of percent returns, rounded to 3 dp.
///
/// Returns zero for empty inputs. The square root is computed via `f64`, which is
/// adequate precision for a reported risk metric; a non-finite or negative
/// intermediate value falls back to zero rather than panicking.
fn std_dev_pct(returns: &[Decimal]) -> Decimal {
    if returns.is_empty() {
        return Decimal::ZERO;
    }
    let n = Decimal::from(returns.len() as u64);
    let sum: Decimal = returns.iter().copied().sum();
    let mean = sum / n;
    let variance: Decimal = returns
        .iter()
        .map(|r| {
            let diff = *r - mean;
            diff * diff
        })
        .sum::<Decimal>()
        / n;

    let variance_f64 = match variance.to_f64() {
        Some(v) if v.is_finite() && v >= 0.0 => v,
        _ => return Decimal::ZERO,
    };
    match Decimal::from_f64_retain(variance_f64.sqrt()) {
        Some(sd) => sd.round_dp(3),
        None => Decimal::ZERO,
    }
}
