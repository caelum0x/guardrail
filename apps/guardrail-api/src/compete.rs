//! Track-1 competition readiness endpoint.
//!
//! Read-only and side-effect free. Reports whether the agent is registered
//! against the competition contract, how many assets are eligible, and whether
//! the daily-trade requirement has been satisfied based on the recent event
//! log. Never panics: all fallible reads degrade to safe defaults.

use axum::{extract::State, Json};
use event_store::AgentEvent;
use serde_json::{json, Value};

use crate::routes::AppState;

const COMPETITION_CONTRACT: &str = "0x212c61b9b72c95d95bf29cf032f5e5635629aed5";
const COMPETITION_CONTRACT_BSCTRACE: &str =
    "https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5";
const ELIGIBLE_ASSETS_PATH: &str = "configs/eligible_assets.bsc.json";
const RECENT_LIMIT: usize = 200;

pub async fn compete(State(state): State<AppState>) -> Json<Value> {
    let events = state.recent_events(RECENT_LIMIT).unwrap_or_default();

    let eligible_assets = market_data::Universe::load(ELIGIBLE_ASSETS_PATH)
        .map(|universe| universe.enabled().len())
        .unwrap_or(0);

    let competition_tx = events.iter().find_map(|event| {
        if matches!(event.event_type, AgentEvent::TxConfirmed) {
            event
                .payload_json
                .get("competition_tx")
                .and_then(Value::as_str)
        } else {
            None
        }
    });

    let registered = competition_tx.is_some();

    let daily_trade_satisfied = events
        .iter()
        .any(|event| matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied));

    let confirmed_trades = events
        .iter()
        .filter(|event| matches!(event.event_type, AgentEvent::TxConfirmed))
        .count();

    let kill_switch = events
        .iter()
        .any(|event| matches!(event.event_type, AgentEvent::KillSwitchTriggered));

    Json(json!({
        "competition_contract": COMPETITION_CONTRACT,
        "competition_contract_bsctrace": COMPETITION_CONTRACT_BSCTRACE,
        "eligible_assets": eligible_assets,
        "registered": registered,
        "competition_tx": competition_tx,
        "daily_trade_satisfied": daily_trade_satisfied,
        "confirmed_trades": confirmed_trades,
        "kill_switch": kill_switch
    }))
}
