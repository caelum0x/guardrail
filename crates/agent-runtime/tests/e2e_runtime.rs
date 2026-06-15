//! End-to-end integration test for the agent runtime.
//!
//! Drives a single full trading cycle in paper mode (mock CMC + mock TWAK),
//! then asserts the SQLite event log and the on-disk run report reflect a
//! complete run: an `AgentStarted` event, a `PortfolioReconciled` event, an
//! `AgentReportPublished` event, and a JSON report containing a `run_id`.
//!
//! Tests run with CWD = the crate directory, so all repo files are referenced
//! via absolute paths built from `CARGO_MANIFEST_DIR` joined with `../../`.

use std::path::{Path, PathBuf};

use agent_runtime::AgentRuntime;
use common::Settings;
use event_store::{AgentEvent, SqliteEventRepository};

/// Canonicalized absolute path to the repository root.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .canonicalize()
        .expect("failed to canonicalize repo root from CARGO_MANIFEST_DIR")
}

/// Absolute path to a file under `configs/`.
fn config_path(name: &str) -> PathBuf {
    repo_root().join("configs").join(name)
}

#[tokio::test]
async fn runtime_runs_one_cycle_and_persists_events_and_report() {
    // --- Unique temp file locations (avoid collisions across test runs) ----
    let unique = format!(
        "{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let temp_dir = std::env::temp_dir().join(format!("guardrail_e2e_{unique}"));
    std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let db_path = temp_dir.join(format!("e2e_{unique}.db"));
    let report_path = temp_dir.join("e2e_report.json");

    // --- Load settings and override storage/policy to absolute paths -------
    let paper_toml = config_path("paper.toml");
    let mut settings = Settings::load(paper_toml.to_str().expect("paper.toml path is valid UTF-8"))
        .expect("failed to load paper.toml settings");

    settings.app.database_url = format!(
        "sqlite://{}",
        db_path.to_str().expect("db path is valid UTF-8")
    );
    settings.risk.policy_path = config_path("risk_policy.paper.json")
        .to_str()
        .expect("risk policy path is valid UTF-8")
        .to_string();

    // --- Drive a single, bounded cycle via env -----------------------------
    let universe_path = config_path("eligible_assets.bsc.json");
    // SAFETY-of-test note: this is a single #[tokio::test], so mutating global
    // process env here is acceptable (no concurrent tests share this state).
    std::env::set_var("GUARDRAIL_CYCLES", "1");
    std::env::set_var(
        "GUARDRAIL_UNIVERSE",
        universe_path
            .to_str()
            .expect("universe path is valid UTF-8"),
    );
    std::env::set_var(
        "GUARDRAIL_REPORT",
        report_path.to_str().expect("report path is valid UTF-8"),
    );

    // --- Run -------------------------------------------------------------
    AgentRuntime::new(settings)
        .run()
        .await
        .expect("agent runtime run() should succeed in paper mode");

    // --- Assert: SQLite event log has the key lifecycle events ------------
    let repo = SqliteEventRepository::open(&db_path)
        .expect("failed to open the SQLite event repository written by the run");
    let events = repo.recent(1000).expect("failed to read recent events");

    assert!(
        !events.is_empty(),
        "expected the run to persist at least one event"
    );

    let has = |target: &AgentEvent| {
        events
            .iter()
            .any(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(target))
    };

    assert!(
        has(&AgentEvent::AgentStarted),
        "expected an AgentStarted event in the persisted log; got: {:?}",
        event_names(&events)
    );
    assert!(
        has(&AgentEvent::AgentReportPublished),
        "expected an AgentReportPublished event in the persisted log; got: {:?}",
        event_names(&events)
    );
    assert!(
        has(&AgentEvent::PortfolioReconciled),
        "expected a PortfolioReconciled event in the persisted log; got: {:?}",
        event_names(&events)
    );

    // --- Assert: run report exists and is valid JSON with a run_id --------
    assert!(
        report_path.exists(),
        "expected the GUARDRAIL_REPORT file to be written at {}",
        report_path.display()
    );
    let report_raw =
        std::fs::read_to_string(&report_path).expect("failed to read the run report file");
    let report: serde_json::Value =
        serde_json::from_str(&report_raw).expect("run report should be valid JSON");
    let run_id = report
        .get("run_id")
        .and_then(|v| v.as_str())
        .expect("run report JSON should contain a string \"run_id\" key");
    assert!(
        !run_id.is_empty(),
        "run_id in the report should not be empty"
    );

    // The persisted events should belong to the same run as the report.
    assert!(
        events.iter().any(|e| e.run_id == run_id),
        "expected persisted events to share the report's run_id ({run_id})"
    );

    // --- Cleanup ----------------------------------------------------------
    drop(repo);
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Tidy the env we set so it does not leak to other test binaries.
    std::env::remove_var("GUARDRAIL_CYCLES");
    std::env::remove_var("GUARDRAIL_UNIVERSE");
    std::env::remove_var("GUARDRAIL_REPORT");
}

/// Human-readable list of event variant names for assertion messages.
fn event_names(events: &[event_store::StoredEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| {
            serde_json::to_value(&e.event_type)
                .ok()
                .and_then(|v| v.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "<unknown>".to_string())
        })
        .collect()
}
