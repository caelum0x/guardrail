//! Append-only event log for agent decisions, risk checks, and execution proof.

pub mod db;
pub mod event;
pub mod export;
pub mod migrations;
pub mod projections;
pub mod queries;
pub mod repository;
pub mod store;

pub use event::{AgentEvent, StoredEvent};
pub use repository::{EventRepository, SqliteEventRepository};
pub use store::{SqliteEventStore, StoreError};
