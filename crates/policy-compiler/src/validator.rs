//! Policy validation. A policy — whether hand-written JSON or compiled from a
//! natural-language mandate — must pass these invariants before it can bind the
//! live runtime. This is the safety backstop on the "NL in" path.

use risk_engine::RiskPolicy;
use rust_decimal::Decimal;

fn in_range(label: &str, v: Decimal, lo: Decimal, hi: Decimal) -> anyhow::Result<()> {
    if v < lo || v > hi {
        anyhow::bail!("{label} = {v} out of range [{lo}, {hi}]");
    }
    Ok(())
}

/// Validate a risk policy for internal consistency and safe bounds.
pub fn validate_policy(policy: &RiskPolicy) -> anyhow::Result<()> {
    let zero = Decimal::ZERO;
    let hundred = Decimal::from(100);

    in_range(
        "max_total_drawdown_pct",
        policy.max_total_drawdown_pct,
        zero,
        hundred,
    )?;
    in_range(
        "max_daily_drawdown_pct",
        policy.max_daily_drawdown_pct,
        zero,
        hundred,
    )?;
    in_range("max_position_pct", policy.max_position_pct, zero, hundred)?;
    in_range(
        "max_new_position_pct",
        policy.max_new_position_pct,
        zero,
        hundred,
    )?;
    in_range(
        "min_stable_reserve_pct",
        policy.min_stable_reserve_pct,
        zero,
        hundred,
    )?;
    in_range(
        "max_slippage_pct",
        policy.max_slippage_pct,
        zero,
        Decimal::from(10),
    )?;
    in_range(
        "kill_switch_drawdown_pct",
        policy.kill_switch_drawdown_pct,
        zero,
        hundred,
    )?;

    if policy.max_daily_drawdown_pct > policy.max_total_drawdown_pct {
        anyhow::bail!(
            "daily drawdown cap ({}%) cannot exceed total drawdown cap ({}%)",
            policy.max_daily_drawdown_pct,
            policy.max_total_drawdown_pct
        );
    }
    if policy.kill_switch_drawdown_pct < policy.max_total_drawdown_pct {
        anyhow::bail!(
            "kill switch ({}%) must trigger at or beyond the total drawdown cap ({}%)",
            policy.kill_switch_drawdown_pct,
            policy.max_total_drawdown_pct
        );
    }
    if policy.max_new_position_pct > policy.max_position_pct {
        anyhow::bail!(
            "new-position cap ({}%) cannot exceed max-position cap ({}%)",
            policy.max_new_position_pct,
            policy.max_position_pct
        );
    }
    if policy.execution_layer != "twak_only" {
        anyhow::bail!(
            "execution_layer must be 'twak_only', got '{}'",
            policy.execution_layer
        );
    }
    if policy.allowed_assets.is_empty() {
        anyhow::bail!("allowed_assets must list at least one eligible asset");
    }
    if policy.daily_trade_requirement.enabled
        && policy.daily_trade_requirement.min_trades_per_day == 0
    {
        anyhow::bail!("daily trade requirement is enabled but min_trades_per_day is 0");
    }
    Ok(())
}
