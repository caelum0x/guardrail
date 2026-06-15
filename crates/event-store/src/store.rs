//! Production-grade append-only event store backed by SQLite.
//!
//! [`SqliteEventStore`] owns a rusqlite [`Connection`] and persists agent
//! events to an append-only `events` table. The schema (mirroring the DDL in
//! `migrations/*.sql`) is created on open with `CREATE TABLE IF NOT EXISTS`,
//! so opening an existing database is idempotent.

use crate::event::{AgentEvent, StoredEvent};
use common::ids;
use rusqlite::{params, Connection};
use serde_json::Value;

/// Typed errors produced by [`SqliteEventStore`].
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The underlying SQLite engine returned an error.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// (De)serialization of an event type or payload failed.
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// An `AgentEvent` did not serialize to its expected snake_case string.
    #[error("agent event did not serialize to a string")]
    EventTypeEncoding,
}

/// Inlined DDL mirroring `migrations/0001_init.sql`, `0003_trade_events.sql`,
/// and `0004_risk_events.sql`. Applied with `CREATE TABLE IF NOT EXISTS` so it
/// is safe to run on every open.
const SCHEMA_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    mode TEXT NOT NULL,
    strategy_version TEXT NOT NULL,
    policy_hash TEXT NOT NULL,
    wallet_address TEXT
);

CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS trade_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    from_symbol TEXT NOT NULL,
    to_symbol TEXT NOT NULL,
    amount_usd REAL NOT NULL,
    status TEXT NOT NULL,
    quote_json TEXT,
    risk_decision_json TEXT,
    tx_hash TEXT,
    reason TEXT
);

CREATE TABLE IF NOT EXISTS risk_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    check_name TEXT NOT NULL,
    status TEXT NOT NULL,
    reason TEXT,
    payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_events_run_timestamp
    ON events(run_id, timestamp);
"#;

/// Append-only event log backed by an owned SQLite connection.
pub struct SqliteEventStore {
    conn: Connection,
}

impl SqliteEventStore {
    /// Opens (or creates) the database at `path` and ensures the schema exists.
    ///
    /// Pass `":memory:"` or `"file::memory:?cache=shared"` for an in-memory
    /// database. The schema is applied idempotently via `CREATE TABLE IF NOT
    /// EXISTS`, so opening an existing database never destroys data.
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    /// Applies the inlined schema. Idempotent.
    fn init_schema(&self) -> Result<(), StoreError> {
        self.conn.execute_batch(SCHEMA_DDL)?;
        Ok(())
    }

    /// Appends a single event to the log.
    ///
    /// The event type is stored as its snake_case string, the payload as JSON
    /// text, the timestamp as RFC3339, and the id as a fresh UUID.
    pub fn append(
        &self,
        run_id: impl Into<String>,
        event_type: AgentEvent,
        payload_json: Value,
    ) -> anyhow::Result<()> {
        let id = ids::new_id();
        let run_id = run_id.into();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let type_name = event_type_name(&event_type)?;
        let payload_text = serde_json::to_string(&payload_json).map_err(StoreError::from)?;

        self.conn.execute(
            "INSERT INTO events (id, run_id, timestamp, event_type, payload_json) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, run_id, timestamp, type_name, payload_text],
        )?;
        Ok(())
    }

    /// Returns all events for `run_id` ordered chronologically (oldest first).
    pub fn by_run(&self, run_id: &str) -> anyhow::Result<Vec<StoredEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, run_id, timestamp, event_type, payload_json \
             FROM events WHERE run_id = ?1 ORDER BY timestamp ASC, id ASC",
        )?;

        let rows = stmt.query_map(params![run_id], |row| {
            Ok(RawEvent {
                id: row.get(0)?,
                run_id: row.get(1)?,
                timestamp: row.get(2)?,
                event_type: row.get(3)?,
                payload_json: row.get(4)?,
            })
        })?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?.into_stored()?);
        }
        Ok(events)
    }

    /// Returns the total number of stored events.
    pub fn count(&self) -> anyhow::Result<i64> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(total)
    }
}

/// Raw database row before deserialization into a [`StoredEvent`].
struct RawEvent {
    id: String,
    run_id: String,
    timestamp: String,
    event_type: String,
    payload_json: String,
}

impl RawEvent {
    fn into_stored(self) -> Result<StoredEvent, StoreError> {
        Ok(StoredEvent {
            id: self.id,
            run_id: self.run_id,
            timestamp: self.timestamp,
            event_type: event_type_from_name(&self.event_type)?,
            payload_json: serde_json::from_str(&self.payload_json)?,
        })
    }
}

/// Serializes an [`AgentEvent`] to its snake_case string representation.
fn event_type_name(event_type: &AgentEvent) -> Result<String, StoreError> {
    serde_json::to_value(event_type)?
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or(StoreError::EventTypeEncoding)
}

/// Parses an [`AgentEvent`] from its snake_case string representation.
fn event_type_from_name(name: &str) -> Result<AgentEvent, StoreError> {
    Ok(serde_json::from_value(Value::String(name.to_string()))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> SqliteEventStore {
        SqliteEventStore::open(":memory:").expect("in-memory sqlite opens")
    }

    #[test]
    fn open_creates_schema_and_starts_empty() {
        let store = store();
        assert_eq!(store.count().expect("count"), 0);
    }

    #[test]
    fn append_then_count_reflects_inserts() {
        let store = store();
        store
            .append(
                "run-1",
                AgentEvent::AgentStarted,
                json!({ "mode": "paper" }),
            )
            .expect("append");
        store
            .append("run-1", AgentEvent::RiskApproved, json!({ "order": "o-1" }))
            .expect("append");

        assert_eq!(store.count().expect("count"), 2);
    }

    #[test]
    fn by_run_returns_only_matching_run_in_order() {
        let store = store();
        store
            .append("run-a", AgentEvent::AgentStarted, json!({ "step": 1 }))
            .expect("append");
        store
            .append("run-b", AgentEvent::AssetScored, json!({ "step": 99 }))
            .expect("append");
        store
            .append("run-a", AgentEvent::OrderProposed, json!({ "step": 2 }))
            .expect("append");

        let events = store.by_run("run-a").expect("by_run");
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0].event_type, AgentEvent::AgentStarted));
        assert!(matches!(events[1].event_type, AgentEvent::OrderProposed));
        assert_eq!(events[0].payload_json["step"], 1);
        assert!(events.iter().all(|e| e.run_id == "run-a"));
    }

    #[test]
    fn by_run_unknown_run_is_empty() {
        let store = store();
        store
            .append("run-1", AgentEvent::AgentStarted, json!({}))
            .expect("append");
        assert!(store.by_run("missing").expect("by_run").is_empty());
    }

    #[test]
    fn round_trips_event_type_and_payload() {
        let store = store();
        store
            .append(
                "run-x",
                AgentEvent::KillSwitchTriggered,
                json!({ "reason": "drawdown", "value": 42 }),
            )
            .expect("append");

        let events = store.by_run("run-x").expect("by_run");
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0].event_type,
            AgentEvent::KillSwitchTriggered
        ));
        assert_eq!(events[0].payload_json["reason"], "drawdown");
        assert_eq!(events[0].payload_json["value"], 42);
        assert!(!events[0].id.is_empty());
        assert!(!events[0].timestamp.is_empty());
    }
}
