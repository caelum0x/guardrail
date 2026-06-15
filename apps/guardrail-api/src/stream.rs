//! Real-time Server-Sent Events feed of the append-only event log.
//!
//! `GET /stream` opens a long-lived `text/event-stream` connection that *tails*
//! the SQLite event log written by the trading agent in a separate process. On
//! connect it replays the most recent events (oldest-first) so a fresh client
//! immediately has context, then on a fixed interval it polls the log and emits
//! any events that appeared since the last poll. A periodic keep-alive comment
//! keeps proxies and idle connections from closing the stream.
//!
//! Design notes:
//!
//! * The event store only exposes `recent(limit)` (newest-first, ordered by
//!   `(timestamp DESC, id DESC)`). Event ids are opaque strings (not a
//!   monotonic counter), so "newer than last seen" is computed against the
//!   composite `(timestamp, id)` ordering key rather than a numeric id. This is
//!   the same total order the store sorts by, so it is stable and works across
//!   processes (the API only ever reads; the agent only ever writes).
//! * The stream is built from a `tokio::time::interval` so it never blocks the
//!   runtime; the per-tick DB read is cheap and bounded by `TAIL_LIMIT`.
//! * A missing or empty database never panics: read errors degrade to "no new
//!   events" and the keep-alive continues, so the client stays connected and
//!   recovers automatically once the agent starts writing.
//! * When the client disconnects axum drops the response stream, which drops
//!   the underlying future and ends the loop cleanly — no background task is
//!   leaked.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::{self, Stream};
use futures::StreamExt;
use serde_json::json;

use crate::routes::AppState;

/// How many events to consider on each poll. Generous enough to never miss a
/// burst between ticks while keeping each read bounded.
const TAIL_LIMIT: usize = 500;

/// How many events to replay to a freshly-connected client.
const REPLAY_LIMIT: usize = 50;

/// Polling cadence for new events.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Keep-alive comment cadence (must be larger than the poll interval so it only
/// fires during genuinely idle stretches).
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

/// Composite ordering key matching the store's `ORDER BY timestamp DESC, id DESC`.
///
/// Comparing two events by this key yields the same total order the repository
/// uses, so "strictly newer than the last emitted event" is a simple `>`.
type Cursor = (String, String);

fn cursor_of(event: &event_store::StoredEvent) -> Cursor {
    (event.timestamp.clone(), event.id.clone())
}

/// Internal state threaded through the stream generator.
struct TailState {
    app: AppState,
    /// Highest `(timestamp, id)` already emitted, or `None` before the first
    /// event has been sent.
    last: Option<Cursor>,
    interval: tokio::time::Interval,
    /// Whether the initial replay batch has been delivered yet.
    primed: bool,
}

/// Serialize a stored event into an SSE `data:` frame carrying its JSON.
fn event_frame(stored: &event_store::StoredEvent) -> Event {
    // `StoredEvent` is `Serialize`; fall back to a minimal envelope if (very
    // unexpectedly) serialization fails, so a single bad row never kills the
    // stream.
    let data = serde_json::to_string(stored).unwrap_or_else(|error| {
        json!({
            "id": stored.id,
            "error": format!("serialize failed: {error}"),
        })
        .to_string()
    });
    Event::default().event("agent_event").data(data)
}

/// Read events newer than `last` (chronological, oldest-first). Read failures
/// and an empty/missing DB both yield an empty list — never an error or panic.
fn fetch_new(app: &AppState, last: &Option<Cursor>, limit: usize) -> Vec<event_store::StoredEvent> {
    let mut events = app.recent_events(limit).unwrap_or_default();
    // `recent_events` is newest-first; emit oldest-first for a natural feed.
    events.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| a.id.cmp(&b.id))
    });
    match last {
        Some(cursor) => events
            .into_iter()
            .filter(|event| &cursor_of(event) > cursor)
            .collect(),
        None => events,
    }
}

/// `GET /stream` — long-lived SSE feed that tails the event log.
pub async fn stream(State(app): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut interval = tokio::time::interval(POLL_INTERVAL);
    // If a tick is missed (slow client / scheduler), skip rather than burst.
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let initial = TailState {
        app,
        last: None,
        interval,
        primed: false,
    };

    let sse_stream = stream::unfold(initial, |mut state| async move {
        // First poll delivers the replay batch; subsequent polls deliver only
        // events newer than the cursor.
        loop {
            let limit = if state.primed { TAIL_LIMIT } else { REPLAY_LIMIT };
            let fresh = fetch_new(&state.app, &state.last, limit);

            if let Some(newest) = fresh.last() {
                state.last = Some(cursor_of(newest));
            }
            state.primed = true;

            if !fresh.is_empty() {
                let frames: Vec<Result<Event, Infallible>> =
                    fresh.iter().map(|event| Ok(event_frame(event))).collect();
                return Some((stream::iter(frames), state));
            }

            // Nothing new this tick — wait and poll again. The keep-alive
            // configured on the `Sse` handles idle-connection liveness, so an
            // idle stretch here simply yields no frames.
            state.interval.tick().await;
        }
    })
    .flatten();

    Sse::new(sse_stream).keep_alive(
        KeepAlive::new()
            .interval(KEEPALIVE_INTERVAL)
            .text("keep-alive"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use event_store::{AgentEvent, StoredEvent};
    use serde_json::Value;

    fn ev(ts: &str, id: &str) -> StoredEvent {
        StoredEvent {
            id: id.to_string(),
            run_id: "run".to_string(),
            timestamp: ts.to_string(),
            event_type: AgentEvent::AgentStarted,
            payload_json: Value::Null,
        }
    }

    #[test]
    fn cursor_orders_by_timestamp_then_id() {
        let a = cursor_of(&ev("2026-01-01T00:00:00Z", "a"));
        let b = cursor_of(&ev("2026-01-01T00:00:00Z", "b"));
        let c = cursor_of(&ev("2026-01-01T00:00:01Z", "a"));
        assert!(a < b, "same timestamp orders by id");
        assert!(b < c, "later timestamp wins regardless of id");
    }

    #[test]
    fn event_frame_carries_serialized_json() {
        let frame = event_frame(&ev("2026-01-01T00:00:00Z", "x"));
        // Render the frame to its on-the-wire form and assert it is a data line
        // containing the event id.
        let wire = format!("{frame:?}");
        assert!(wire.contains('x'), "frame should reference the event id");
    }
}
