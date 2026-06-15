use crate::approval::RiskDecision;
use common::OrderIntent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAuditRecord {
    pub order_id: String,
    pub decision: RiskDecision,
}

impl RiskAuditRecord {
    pub fn from_decision(intent: &OrderIntent, decision: RiskDecision) -> Self {
        Self {
            order_id: intent.id.clone(),
            decision,
        }
    }
}
