use crate::policy::RiskPolicy;

pub fn daily_requirement_enabled(policy: &RiskPolicy) -> bool {
    policy.daily_trade_requirement.enabled
}
