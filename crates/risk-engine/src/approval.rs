use crate::checks;
use crate::policy::RiskPolicy;
use common::{ids, Decimal, OrderIntent, QuoteSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskDecision {
    Approved,
    Rejected {
        reasons: Vec<String>,
    },
    Clipped {
        new_amount_usd: Decimal,
        reasons: Vec<String>,
    },
}

impl RiskDecision {
    pub fn is_approved(&self) -> bool {
        matches!(self, RiskDecision::Approved)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovedOrder {
    pub id: String,
    pub intent: OrderIntent,
    pub approved_amount_usd: Decimal,
    pub decision: RiskDecision,
}

#[derive(Debug, Clone)]
pub struct RiskContext {
    pub nav_usd: Decimal,
    pub stable_reserve_pct: Decimal,
    pub total_drawdown_pct: Decimal,
    pub daily_drawdown_pct: Decimal,
    pub target_position_pct: Decimal,
    pub security_flags: Vec<String>,
}

impl RiskContext {
    pub fn empty(nav_usd: Decimal) -> Self {
        Self {
            nav_usd,
            stable_reserve_pct: Decimal::from(100),
            total_drawdown_pct: Decimal::ZERO,
            daily_drawdown_pct: Decimal::ZERO,
            target_position_pct: Decimal::ZERO,
            security_flags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RiskEngine {
    policy: RiskPolicy,
}

impl RiskEngine {
    pub fn new(policy: RiskPolicy) -> Self {
        Self { policy }
    }

    pub fn policy(&self) -> &RiskPolicy {
        &self.policy
    }

    pub fn pre_trade(&self, intent: &OrderIntent, ctx: &RiskContext) -> RiskDecision {
        let mut reasons = checks::run_pre_trade_checks(&self.policy, intent, ctx);
        if !reasons.is_empty() {
            return RiskDecision::Rejected { reasons };
        }

        let max_new_usd = ctx.nav_usd * self.policy.max_new_position_pct / Decimal::from(100);
        if intent.amount_usd > max_new_usd && max_new_usd > Decimal::ZERO {
            reasons.push(format!(
                "order clipped to max_new_position_pct {}%",
                self.policy.max_new_position_pct
            ));
            return RiskDecision::Clipped {
                new_amount_usd: max_new_usd,
                reasons,
            };
        }

        RiskDecision::Approved
    }

    pub fn final_quote_check(
        &self,
        intent: &OrderIntent,
        ctx: &RiskContext,
        quote: &QuoteSummary,
    ) -> RiskDecision {
        let mut reasons = checks::run_pre_trade_checks(&self.policy, intent, ctx);
        reasons.extend(checks::slippage::check(&self.policy, quote));
        reasons.extend(checks::liquidity::check_quote(quote));

        if reasons.is_empty() {
            RiskDecision::Approved
        } else {
            RiskDecision::Rejected { reasons }
        }
    }

    pub fn approve(
        &self,
        intent: OrderIntent,
        ctx: &RiskContext,
        quote: &QuoteSummary,
    ) -> Result<ApprovedOrder, RiskDecision> {
        let pre_trade_decision = self.pre_trade(&intent, ctx);
        let (approved_amount_usd, recorded_decision) = match pre_trade_decision {
            RiskDecision::Approved => Ok((intent.amount_usd, RiskDecision::Approved)),
            RiskDecision::Clipped {
                new_amount_usd,
                reasons,
            } => Ok((
                new_amount_usd,
                RiskDecision::Clipped {
                    new_amount_usd,
                    reasons,
                },
            )),
            rejected => Err(rejected),
        }?;

        let mut checked_intent = intent.clone();
        checked_intent.amount_usd = approved_amount_usd;

        match self.final_quote_check(&checked_intent, ctx, quote) {
            RiskDecision::Approved => Ok(ApprovedOrder {
                id: ids::new_id(),
                approved_amount_usd,
                intent,
                decision: recorded_decision,
            }),
            rejected => Err(rejected),
        }
    }
}
