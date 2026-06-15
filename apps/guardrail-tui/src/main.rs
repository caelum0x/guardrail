//! Guardrail Alpha — terminal cockpit.
//!
//! A dependency-light polling dashboard that renders the live run report and
//! event totals to the terminal. It refreshes a fixed number of times and then
//! exits so it terminates cleanly in demos and CI runs.
//!
//! Configuration (all via environment variables, with safe defaults):
//! - `GUARDRAIL_REPORT`      path to the run report JSON (default `data/run_report.json`)
//! - `DATABASE_URL`          SQLite URL/path (default `sqlite://data/guardrail_alpha.db`)
//! - `GUARDRAIL_TUI_REFRESHES` number of refresh cycles (default `3`)
//!
//! The cockpit never panics: if the report or database are absent or malformed,
//! placeholders are rendered instead.

use std::time::Duration;

use event_store::{SqliteEventRepository, StoredEvent};
use serde_json::Value;

mod alerts;
mod positions;
mod regime;
mod render;
mod report;
mod risk;
mod totals;

use render::Screen;
use report::RunReport;
use totals::EventTotals;

const DEFAULT_REPORT_PATH: &str = "data/run_report.json";
const DEFAULT_DB_URL: &str = "sqlite://data/guardrail_alpha.db";
const DEFAULT_POLICY_PATH: &str = "configs/risk_policy.paper.json";
const DEFAULT_REFRESHES: u32 = 3;
const RECENT_EVENT_LIMIT: usize = 500;
const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

fn main() {
    let report_path = env_or("GUARDRAIL_REPORT", DEFAULT_REPORT_PATH);
    let db_path = resolve_db_path();
    let policy_path = env_or("GUARDRAIL_RISK_POLICY", DEFAULT_POLICY_PATH);
    let refreshes = resolve_refreshes();

    for cycle in 0..refreshes {
        let report = RunReport::load(&report_path);
        let events = load_recent_events(&db_path);
        let totals = EventTotals::from_recent(&events);

        let screen = Screen::new(
            &report,
            &totals,
            &events,
            &policy_path,
            cycle + 1,
            refreshes,
        );
        // Writing to stdout is best-effort; a broken pipe must not crash the cockpit.
        print!("{}", screen.render());
        let _ = std::io::Write::flush(&mut std::io::stdout());

        if cycle + 1 < refreshes {
            std::thread::sleep(REFRESH_INTERVAL);
        }
    }
}

/// Reads an environment variable, falling back to a default when unset or empty.
fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => default.to_string(),
    }
}

/// Resolves the SQLite database path from `DATABASE_URL`, stripping the
/// `sqlite://` prefix if present.
fn resolve_db_path() -> String {
    let raw = env_or("DATABASE_URL", DEFAULT_DB_URL);
    raw.strip_prefix("sqlite://")
        .map(ToOwned::to_owned)
        .unwrap_or(raw)
}

/// Resolves the number of refresh cycles, falling back to the default when the
/// value is unset, empty, or unparsable. A value of zero is bumped to one so the
/// cockpit always renders at least once.
fn resolve_refreshes() -> u32 {
    let parsed = std::env::var("GUARDRAIL_TUI_REFRESHES")
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .unwrap_or(DEFAULT_REFRESHES);
    parsed.max(1)
}

/// Loads the most recent events from the SQLite event store (newest-first),
/// returning `None` if the database is missing or unreadable so downstream
/// panels can render placeholders.
fn load_recent_events(db_path: &str) -> Option<Vec<StoredEvent>> {
    if !std::path::Path::new(db_path).exists() {
        return None;
    }
    let repo = SqliteEventRepository::open(db_path).ok()?;
    repo.recent(RECENT_EVENT_LIMIT).ok()
}

/// Extracts a string field from a JSON object, returning a placeholder when
/// absent. Numbers are rendered as-is so decimal strings or numeric JSON both
/// display.
fn field_str(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => "—".to_string(),
    }
}
