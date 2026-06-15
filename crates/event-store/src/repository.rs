use crate::event::{AgentEvent, StoredEvent};
use common::ids;
use rusqlite::{params, Connection};
use serde_json::Value;

#[derive(Debug, Default)]
pub struct EventRepository {
    events: Vec<StoredEvent>,
}

impl EventRepository {
    pub fn new_memory() -> Self {
        Self::default()
    }

    pub fn append(
        &mut self,
        run_id: impl Into<String>,
        event_type: AgentEvent,
        payload_json: Value,
    ) {
        self.events.push(StoredEvent {
            id: ids::new_id(),
            run_id: run_id.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type,
            payload_json,
        });
    }

    pub fn all(&self) -> &[StoredEvent] {
        &self.events
    }
}

pub struct SqliteEventRepository {
    conn: Connection,
}

impl SqliteEventRepository {
    pub fn open(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let repo = Self {
            conn: Connection::open(path)?,
        };
        repo.initialize()?;
        Ok(repo)
    }

    pub fn new_in_memory() -> anyhow::Result<Self> {
        let repo = Self {
            conn: Connection::open_in_memory()?,
        };
        repo.initialize()?;
        Ok(repo)
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload_json TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_events_run_timestamp
                ON events(run_id, timestamp);
            "#,
        )?;
        Ok(())
    }

    pub fn append(
        &self,
        run_id: impl Into<String>,
        event_type: AgentEvent,
        payload_json: Value,
    ) -> anyhow::Result<StoredEvent> {
        let event = StoredEvent {
            id: ids::new_id(),
            run_id: run_id.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type,
            payload_json,
        };

        self.conn.execute(
            r#"
            INSERT INTO events (id, run_id, timestamp, event_type, payload_json)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                event.id,
                event.run_id,
                event.timestamp,
                event_type_name(&event.event_type)?,
                serde_json::to_string(&event.payload_json)?,
            ],
        )?;

        Ok(event)
    }

    pub fn recent(&self, limit: usize) -> anyhow::Result<Vec<StoredEvent>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, run_id, timestamp, event_type, payload_json
            FROM events
            ORDER BY timestamp DESC, id DESC
            LIMIT ?1
            "#,
        )?;

        let rows = stmt.query_map([limit as i64], |row| {
            let event_type: String = row.get(3)?;
            let payload_json: String = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                event_type,
                payload_json,
            ))
        })?;

        let mut events = Vec::new();
        for row in rows {
            let (id, run_id, timestamp, event_type, payload_json) = row?;
            events.push(StoredEvent {
                id,
                run_id,
                timestamp,
                event_type: event_type_from_name(&event_type)?,
                payload_json: serde_json::from_str(&payload_json)?,
            });
        }

        Ok(events)
    }
}

fn event_type_name(event_type: &AgentEvent) -> anyhow::Result<String> {
    let value = serde_json::to_value(event_type)?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("agent event did not serialize to a string"))
}

fn event_type_from_name(name: &str) -> anyhow::Result<AgentEvent> {
    Ok(serde_json::from_value(Value::String(name.to_string()))?)
}
