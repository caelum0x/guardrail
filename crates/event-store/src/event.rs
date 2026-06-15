use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEvent {
    AgentStarted,
    MarketSnapshotReceived,
    RegimeClassified,
    AssetScored,
    PortfolioTargetComputed,
    OrderProposed,
    RiskApproved,
    RiskRejected,
    RiskClipped,
    TwakQuoteReceived,
    TwakSwapSubmitted,
    TxConfirmed,
    PortfolioReconciled,
    DrawdownThrottleActivated,
    KillSwitchTriggered,
    DailyTradeRequirementSatisfied,
    AgentReportPublished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub id: String,
    pub run_id: String,
    pub timestamp: String,
    pub event_type: AgentEvent,
    pub payload_json: Value,
}
