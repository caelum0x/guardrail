//! Human-readable decision journal endpoint.
//!
//! Reconstructs the verifiable-autonomy narrative directly from the append-only
//! event log: for each decision cycle it tells the story
//! "the agent saw the market -> classified the regime -> scored assets ->
//! proposed orders -> the risk engine approved/clipped/rejected ->
//! trades confirmed -> the book was reconciled".
//!
//! This is the Rust counterpart of `python-lab/guardrail_lab/journal.py`, built
//! over the same SQLite event store. A *cycle* is a contiguous span of events
//! that begins at a `regime_classified` event and runs until the next one;
//! events before the first classification are attributed to a synthetic
//! "startup" cycle so nothing is lost.
//!
//! Read-only and side-effect free. Never panics: a missing or empty log yields
//! an empty journal rather than an error.

use axum::{extract::State, Json};
use event_store::{AgentEvent, StoredEvent};
use serde_json::{json, Value};

use crate::routes::AppState;

const RECENT_LIMIT: usize = 1000;

pub async fn journal(State(state): State<AppState>) -> Json<Value> {
    let mut events = state.recent_events(RECENT_LIMIT).unwrap_or_default();
    // `recent_events` returns newest-first; the journal narrative is chronological.
    events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp).then(a.id.cmp(&b.id)));

    let segments = segment_cycles(&events);
    let cycles: Vec<Value> = segments
        .iter()
        .enumerate()
        .map(|(index, slice)| build_cycle(index + 1, slice))
        .collect();

    let run_ids = distinct_run_ids(&events);
    let confirmed_total: u64 = cycles
        .iter()
        .filter_map(|cycle| cycle.get("confirmed_trades").and_then(Value::as_u64))
        .sum();

    Json(json!({
        "total_events": events.len(),
        "total_cycles": cycles.len(),
        "run_ids": run_ids,
        "confirmed_trades_total": confirmed_total,
        "cycles": cycles,
    }))
}

/// Splits a chronologically-ordered event list into per-cycle slices at each
/// `regime_classified` event. Events preceding the first classification form a
/// leading "startup" slice so no event is dropped.
fn segment_cycles(events: &[StoredEvent]) -> Vec<Vec<&StoredEvent>> {
    let mut segments: Vec<Vec<&StoredEvent>> = Vec::new();
    let mut current: Vec<&StoredEvent> = Vec::new();

    for event in events {
        if matches!(event.event_type, AgentEvent::RegimeClassified) && !current.is_empty() {
            segments.push(std::mem::take(&mut current));
        }
        current.push(event);
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

/// Constructs the narrative-ready facts of a single decision cycle.
fn build_cycle(index: usize, slice: &[&StoredEvent]) -> Value {
    let Some(first) = slice.first() else {
        return json!({ "index": index, "regime": "startup" });
    };
    let last = slice.last().copied().unwrap_or(first);

    let regime = if matches!(first.event_type, AgentEvent::RegimeClassified) {
        first
            .payload_json
            .get("regime")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|raw| !raw.is_empty())
            .unwrap_or("unknown")
            .to_string()
    } else {
        "startup".to_string()
    };

    let mut headline = String::new();
    let mut scored: Vec<(String, f64)> = Vec::new();
    let mut orders: Vec<Value> = Vec::new();
    let (mut approved, mut clipped, mut rejected, mut confirmed) = (0u64, 0u64, 0u64, 0u64);
    let mut reasons: Vec<String> = Vec::new();
    let mut ending_nav: Option<String> = None;
    let mut positions: Option<u64> = None;

    for event in slice {
        let payload = &event.payload_json;
        match event.event_type {
            AgentEvent::PortfolioTargetComputed => {
                if let Some(head) = payload.get("headline").and_then(Value::as_str) {
                    let trimmed = head.trim();
                    if !trimmed.is_empty() {
                        headline = trimmed.to_string();
                    }
                }
            }
            AgentEvent::AssetScored => {
                let symbol = payload.get("symbol").and_then(Value::as_str);
                let score = payload_to_f64(payload.get("score"));
                if let (Some(symbol), Some(score)) = (symbol, score) {
                    let symbol = symbol.trim();
                    if !symbol.is_empty() {
                        scored.push((symbol.to_string(), score));
                    }
                }
            }
            AgentEvent::OrderProposed => {
                orders.push(json!({
                    "from": payload.get("from").and_then(Value::as_str).unwrap_or("?"),
                    "to": payload.get("to").and_then(Value::as_str).unwrap_or("?"),
                    "amount_usd": payload_to_f64(payload.get("amount_usd")),
                }));
            }
            AgentEvent::RiskApproved => approved += 1,
            AgentEvent::RiskClipped => clipped += 1,
            AgentEvent::RiskRejected => {
                rejected += 1;
                if let Some(list) = payload.get("reasons").and_then(Value::as_array) {
                    for reason in list {
                        if let Some(text) = reason.as_str() {
                            let text = text.trim();
                            if !text.is_empty() && !reasons.iter().any(|r| r == text) {
                                reasons.push(text.to_string());
                            }
                        }
                    }
                }
            }
            AgentEvent::TxConfirmed => confirmed += 1,
            AgentEvent::PortfolioReconciled => {
                if let Some(nav) = payload_to_f64(payload.get("nav_usd")) {
                    ending_nav = Some(format!("{nav}"));
                }
                if let Some(pos) = payload.get("positions").and_then(Value::as_u64) {
                    positions = Some(pos);
                }
            }
            _ => {}
        }
    }

    scored.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let top_assets: Vec<Value> = scored
        .iter()
        .map(|(symbol, score)| json!({ "symbol": symbol, "score": score }))
        .collect();

    json!({
        "index": index,
        "run_id": first.run_id,
        "regime": regime,
        "started_at": first.timestamp,
        "ended_at": last.timestamp,
        "headline": headline,
        "top_assets": top_assets,
        "orders": orders,
        "risk": {
            "approved": approved,
            "clipped": clipped,
            "rejected": rejected,
            "rejection_reasons": reasons,
        },
        "confirmed_trades": confirmed,
        "ending_nav": ending_nav,
        "positions": positions,
    })
}

/// Distinct run identifiers in first-seen order.
fn distinct_run_ids(events: &[StoredEvent]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for event in events {
        if !event.run_id.is_empty() && !seen.iter().any(|id| id == &event.run_id) {
            seen.push(event.run_id.clone());
        }
    }
    seen
}

/// Coerces a JSON value (number or numeric string) to `f64`, mirroring the
/// lenient parsing used by the Python journal.
fn payload_to_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_str().and_then(|raw| raw.parse::<f64>().ok()))
    })
}
