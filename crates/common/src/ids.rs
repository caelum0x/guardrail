//! Identifier helpers. IDs are UUID-v4 strings so they serialize cleanly to
//! JSON and SQLite without extra encoding.

use uuid::Uuid;

/// Generate a fresh random identifier.
pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

/// A stable run identifier shared by every event in a single agent run.
pub fn new_run_id() -> String {
    format!("run_{}", Uuid::new_v4().simple())
}
