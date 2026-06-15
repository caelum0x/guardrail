use crate::approval::RiskContext;
use crate::policy::RiskPolicy;

pub fn check(policy: &RiskPolicy, ctx: &RiskContext) -> Vec<String> {
    if ctx.daily_drawdown_pct >= policy.max_daily_drawdown_pct {
        vec![format!(
            "daily drawdown {}% exceeds {}%",
            ctx.daily_drawdown_pct, policy.max_daily_drawdown_pct
        )]
    } else {
        Vec::new()
    }
}
