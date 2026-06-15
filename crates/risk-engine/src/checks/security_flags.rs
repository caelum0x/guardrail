use crate::approval::RiskContext;

pub fn check(ctx: &RiskContext) -> Vec<String> {
    ctx.security_flags
        .iter()
        .map(|flag| format!("security flag present: {flag}"))
        .collect()
}
