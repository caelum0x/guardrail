use crate::approval::RiskContext;
use crate::policy::RiskPolicy;

pub fn check(policy: &RiskPolicy, ctx: &RiskContext) -> Vec<String> {
    if ctx.total_drawdown_pct >= policy.kill_switch_drawdown_pct {
        vec![format!(
            "kill switch drawdown {}% exceeds {}%",
            ctx.total_drawdown_pct, policy.kill_switch_drawdown_pct
        )]
    } else if ctx.total_drawdown_pct >= policy.max_total_drawdown_pct {
        vec![format!(
            "total drawdown {}% exceeds {}%",
            ctx.total_drawdown_pct, policy.max_total_drawdown_pct
        )]
    } else {
        Vec::new()
    }
}
