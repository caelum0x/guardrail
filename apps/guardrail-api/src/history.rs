//! Live NAV equity-curve endpoint.
//!
//! Reads recent `PortfolioReconciled` events from the event log and emits the
//! net asset value (NAV) series in chronological order. Read-only and
//! side-effect free — it never mutates the book or the event log.

use axum::{extract::State, Json};
use event_store::AgentEvent;
use serde_json::{json, Value};

use crate::routes::AppState;

/// Number of recent events to scan when building the NAV series.
const RECENT_LIMIT: usize = 200;

/// Return the NAV equity curve assembled from `PortfolioReconciled` events.
///
/// Response shape: `{ points: [{ timestamp, nav_usd }], count }`.
/// On a read error the response is `{ "error": "..." }`.
pub async fn history(State(state): State<AppState>) -> Json<Value> {
    let events = match state.recent_events(RECENT_LIMIT) {
        Ok(events) => events,
        Err(error) => return Json(json!({ "error": error.to_string() })),
    };

    // `recent()` returns newest-first; reverse for chronological order.
    let points: Vec<Value> = events
        .iter()
        .rev()
        .filter(|event| matches!(event.event_type, AgentEvent::PortfolioReconciled))
        .filter_map(|event| {
            let nav_usd = event.payload_json.get("nav_usd").and_then(Value::as_str)?;
            Some(json!({
                "timestamp": event.timestamp,
                "nav_usd": nav_usd,
            }))
        })
        .collect();

    let count = points.len();
    Json(json!({ "points": points, "count": count }))
}
