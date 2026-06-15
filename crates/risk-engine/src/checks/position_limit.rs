use crate::approval::RiskContext;
use crate::policy::RiskPolicy;

pub fn check(policy: &RiskPolicy, ctx: &RiskContext) -> Vec<String> {
    if ctx.target_position_pct > policy.max_position_pct {
        vec![format!(
            "target position {}% exceeds max {}%",
            ctx.target_position_pct, policy.max_position_pct
        )]
    } else {
        Vec::new()
    }
}
