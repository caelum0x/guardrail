use crate::approval::RiskContext;
use crate::policy::RiskPolicy;
use common::OrderIntent;

pub mod asset_allowlist;
pub mod correlation;
pub mod daily_loss;
pub mod liquidity;
pub mod position_limit;
pub mod security_flags;
pub mod slippage;
pub mod stable_reserve;
pub mod total_drawdown;
pub mod trade_frequency;
pub mod wallet_balance;

pub fn run_pre_trade_checks(
    policy: &RiskPolicy,
    intent: &OrderIntent,
    ctx: &RiskContext,
) -> Vec<String> {
    let mut reasons = Vec::new();
    reasons.extend(asset_allowlist::check(policy, intent));
    reasons.extend(position_limit::check(policy, ctx));
    reasons.extend(daily_loss::check(policy, ctx));
    reasons.extend(total_drawdown::check(policy, ctx));
    reasons.extend(stable_reserve::check(policy, ctx));
    reasons.extend(security_flags::check(ctx));
    reasons
}
