use crate::policy::RiskPolicy;
use common::QuoteSummary;

pub fn check(policy: &RiskPolicy, quote: &QuoteSummary) -> Vec<String> {
    if quote.slippage_pct > policy.max_slippage_pct {
        vec![format!(
            "quote slippage {}% exceeds {}%",
            quote.slippage_pct, policy.max_slippage_pct
        )]
    } else {
        Vec::new()
    }
}
