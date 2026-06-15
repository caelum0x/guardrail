//! Live regime-routed ensemble view, sourced from the event log.
//!
//! Unlike [`crate::ensemble`], which *recomputes* the regime-routed blend from
//! the embedded `skills/ensemble.json` config, this endpoint reports the blend
//! the *running agent actually computed*. On every decision cycle the runtime
//! embeds its live ensemble routing inside the `RegimeClassified` event payload
//! under the `"ensemble"` key (`{ regime, skill_weights }`). This endpoint reads
//! the most recent such event and surfaces that embedded payload verbatim.
//!
//! Read-only and side-effect free. It never panics: if no `RegimeClassified`
//! event carrying an `ensemble` payload exists (empty log, older events, or a
//! cycle where the embedded config failed to parse and `ensemble` is `null`),
//! it degrades to `{ "source": "event-log", "available": false }`.

use axum::{extract::State, Json};
use event_store::{AgentEvent, StoredEvent};
use serde_json::{json, Value};

use crate::routes::AppState;

/// Upper bound on how many recent events we scan for a usable classification.
/// `recent_events` returns newest-first, so the first match is the most recent.
const RECENT_LIMIT: usize = 1000;

/// `GET /ensemble/live` — surface the live per-skill ensemble weights the
/// running agent computed in its most recent regime classification.
pub async fn ensemble_live(State(state): State<AppState>) -> Json<Value> {
    let events = state.recent_events(RECENT_LIMIT).unwrap_or_default();
    Json(extract_live_ensemble(&events))
}

/// Pure extraction core, factored out so it can be unit-tested over sample
/// events without a database or a bound socket.
///
/// `events` is expected newest-first (as returned by `AppState::recent_events`).
/// Returns the live-ensemble response object: a populated payload when the most
/// recent `RegimeClassified` event carries an `ensemble` blend, otherwise the
/// `available: false` degradation envelope.
fn extract_live_ensemble(events: &[StoredEvent]) -> Value {
    events
        .iter()
        .find_map(live_ensemble_from_event)
        .unwrap_or_else(|| json!({ "source": "event-log", "available": false }))
}

/// Build the live-ensemble response from a single event, if it is a
/// `RegimeClassified` event whose payload embeds a non-null `ensemble` object
/// with a `skill_weights` map. Returns `None` otherwise so callers can fall
/// through to the next-most-recent candidate.
fn live_ensemble_from_event(event: &StoredEvent) -> Option<Value> {
    if !matches!(event.event_type, AgentEvent::RegimeClassified) {
        return None;
    }

    let ensemble = event.payload_json.get("ensemble")?;
    if ensemble.is_null() {
        return None;
    }

    // The live per-skill weights the running agent actually computed.
    let skill_weights = ensemble.get("skill_weights")?;
    if !skill_weights.is_object() {
        return None;
    }

    // Prefer the regime recorded inside the ensemble payload; fall back to the
    // top-level classification regime so the field is always populated.
    let regime = ensemble
        .get("regime")
        .and_then(Value::as_str)
        .or_else(|| event.payload_json.get("regime").and_then(Value::as_str))
        .map(str::to_string);

    Some(json!({
        "source": "event-log",
        "regime": regime,
        "skill_weights": skill_weights.clone(),
        "event_timestamp": event.timestamp,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Construct a `StoredEvent` with the given type and payload for testing.
    fn event(event_type: AgentEvent, timestamp: &str, payload: Value) -> StoredEvent {
        StoredEvent {
            id: "evt-id".to_string(),
            run_id: "run-1".to_string(),
            timestamp: timestamp.to_string(),
            event_type,
            payload_json: payload,
        }
    }

    /// Mirror of the runtime's embedded `RegimeClassified` payload shape:
    /// `{ regime, ensemble: { regime, skill_weights } }`.
    fn regime_classified(ts: &str, regime: &str, weights: Value) -> StoredEvent {
        event(
            AgentEvent::RegimeClassified,
            ts,
            json!({
                "regime": regime,
                "ensemble": { "regime": regime, "skill_weights": weights },
            }),
        )
    }

    #[test]
    fn extracts_skill_weights_from_most_recent_classification() {
        // Newest-first ordering, exactly as `recent_events` returns.
        let events = vec![
            regime_classified(
                "2026-06-15T12:00:00Z",
                "breakout",
                json!({ "trend-breakout-momentum": 0.5, "cmc-regime-routed-alpha": 0.3 }),
            ),
            regime_classified(
                "2026-06-15T11:00:00Z",
                "chop",
                json!({ "mean-reversion-chop": 0.5 }),
            ),
        ];

        let out = extract_live_ensemble(&events);
        assert_eq!(out["source"], "event-log");
        assert_eq!(out["regime"], "breakout");
        assert_eq!(out["event_timestamp"], "2026-06-15T12:00:00Z");
        assert_eq!(out["skill_weights"]["trend-breakout-momentum"], 0.5);
        assert_eq!(out["skill_weights"]["cmc-regime-routed-alpha"], 0.3);
        // The degradation marker must be absent on the populated path.
        assert!(out.get("available").is_none());
    }

    #[test]
    fn degrades_when_no_regime_event_exists() {
        let events = vec![event(
            AgentEvent::TxConfirmed,
            "2026-06-15T12:00:00Z",
            json!({ "tx_hash": "0xabc" }),
        )];

        let out = extract_live_ensemble(&events);
        assert_eq!(out, json!({ "source": "event-log", "available": false }));
    }

    #[test]
    fn degrades_on_empty_log() {
        let out = extract_live_ensemble(&[]);
        assert_eq!(out, json!({ "source": "event-log", "available": false }));
    }

    #[test]
    fn skips_classification_with_null_ensemble() {
        // A cycle where the embedded config failed to parse: ensemble is null.
        let null_ensemble = event(
            AgentEvent::RegimeClassified,
            "2026-06-15T12:00:00Z",
            json!({ "regime": "risk_on", "ensemble": Value::Null }),
        );
        // An older, valid classification should be used as the fallback.
        let valid = regime_classified(
            "2026-06-15T11:00:00Z",
            "risk_off",
            json!({ "funding-rate-carry": 0.35 }),
        );

        let out = extract_live_ensemble(&[null_ensemble, valid]);
        assert_eq!(out["regime"], "risk_off");
        assert_eq!(out["skill_weights"]["funding-rate-carry"], 0.35);
    }

    #[test]
    fn falls_back_to_top_level_regime_when_ensemble_regime_absent() {
        let ev = event(
            AgentEvent::RegimeClassified,
            "2026-06-15T12:00:00Z",
            json!({
                "regime": "chop",
                "ensemble": { "skill_weights": { "mean-reversion-chop": 0.5 } },
            }),
        );

        let out = extract_live_ensemble(&[ev]);
        assert_eq!(out["regime"], "chop");
        assert_eq!(out["skill_weights"]["mean-reversion-chop"], 0.5);
    }
}
