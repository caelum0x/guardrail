use crate::policy::RiskPolicy;
use common::OrderIntent;

pub fn check(policy: &RiskPolicy, intent: &OrderIntent) -> Vec<String> {
    let mut reasons = Vec::new();
    if !policy.asset_allowed(&intent.from_symbol) {
        reasons.push(format!("from asset {} is not allowed", intent.from_symbol));
    }
    if !policy.asset_allowed(&intent.to_symbol) {
        reasons.push(format!("to asset {} is not allowed", intent.to_symbol));
    }
    reasons
}
