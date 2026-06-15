use crate::approval::RiskContext;
use crate::policy::RiskPolicy;

pub fn check(policy: &RiskPolicy, ctx: &RiskContext) -> Vec<String> {
    if ctx.stable_reserve_pct < policy.min_stable_reserve_pct {
        vec![format!(
            "stable reserve {}% below required {}%",
            ctx.stable_reserve_pct, policy.min_stable_reserve_pct
        )]
    } else {
        Vec::new()
    }
}
