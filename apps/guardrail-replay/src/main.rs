//! Guardrail Alpha event replay / audit tool.
//!
//! Reads the agent's append-only SQLite event log and renders it for humans:
//! a chronological decision journal, a confirmed-trade table, or a CSV export.
//! Read-only — it never trades and never mutates the log. This is the audit
//! trail that answers "why did it trade, what did it quote, what tx resulted?".

use clap::{Parser, Subcommand};
use event_store::{AgentEvent, SqliteEventRepository, StoredEvent};

const DEFAULT_DB_URL: &str = "sqlite://data/guardrail_alpha.db";
const DEFAULT_LIMIT: usize = 1000;

#[derive(Debug, Parser)]
#[command(
    name = "guardrail-replay",
    about = "Replay and audit the agent event log"
)]
struct Cli {
    /// sqlite:// database URL (or DATABASE_URL env).
    #[arg(long)]
    database_url: Option<String>,
    /// Maximum number of recent events to read.
    #[arg(long, default_value_t = DEFAULT_LIMIT)]
    limit: usize,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print a chronological decision journal.
    Journal,
    /// Print confirmed on-chain swaps.
    Trades,
    /// Export all events as CSV to a path (or stdout with "-").
    ExportCsv { path: String },
    /// Print event-type counts.
    Summary,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db_url = cli
        .database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| DEFAULT_DB_URL.into());
    let path = db_url.strip_prefix("sqlite://").unwrap_or(&db_url);

    let repo = SqliteEventRepository::open(path)
        .map_err(|e| anyhow::anyhow!("failed to open event log at {path}: {e}"))?;
    // `recent` returns newest-first; replay reads oldest-first.
    let mut events = repo.recent(cli.limit)?;
    events.reverse();

    match cli.command {
        Command::Journal => journal(&events),
        Command::Trades => trades(&events),
        Command::Summary => summary(&events),
        Command::ExportCsv { path } => export_csv(&events, &path)?,
    }
    Ok(())
}

fn event_name(e: &AgentEvent) -> String {
    serde_json::to_value(e)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

fn journal(events: &[StoredEvent]) {
    if events.is_empty() {
        println!("(no events recorded)");
        return;
    }
    for e in events {
        println!(
            "{}  {:<28}  {}",
            e.timestamp,
            event_name(&e.event_type),
            compact_json(&e.payload_json)
        );
    }
    println!("\n{} events", events.len());
}

fn trades(events: &[StoredEvent]) {
    let confirmed: Vec<&StoredEvent> = events
        .iter()
        .filter(|e| matches!(e.event_type, AgentEvent::TxConfirmed))
        .collect();
    if confirmed.is_empty() {
        println!("(no confirmed transactions)");
        return;
    }
    println!("{:<26}  {:<18}  detail", "timestamp", "tx_hash");
    for e in &confirmed {
        let tx = e
            .payload_json
            .get("tx_hash")
            .and_then(|v| v.as_str())
            .or_else(|| {
                e.payload_json
                    .get("competition_tx")
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("-");
        let short = if tx.len() > 16 { &tx[..16] } else { tx };
        println!(
            "{:<26}  {:<18}  {}",
            e.timestamp,
            short,
            compact_json(&e.payload_json)
        );
    }
    println!("\n{} confirmed transactions", confirmed.len());
}

fn summary(events: &[StoredEvent]) {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for e in events {
        *counts.entry(event_name(&e.event_type)).or_default() += 1;
    }
    for (name, n) in &counts {
        println!("{n:>5}  {name}");
    }
    println!("\n{} total events", events.len());
}

fn export_csv(events: &[StoredEvent], path: &str) -> anyhow::Result<()> {
    let mut out = String::from("id,run_id,timestamp,event_type,payload\n");
    for e in events {
        out.push_str(&format!(
            "{},{},{},{},{}\n",
            e.id,
            e.run_id,
            e.timestamp,
            event_name(&e.event_type),
            csv_escape(&e.payload_json.to_string()),
        ));
    }
    if path == "-" {
        print!("{out}");
    } else {
        std::fs::write(path, out)?;
        println!("wrote {} events to {path}", events.len());
    }
    Ok(())
}

/// Compact one-line JSON for journal display.
fn compact_json(v: &serde_json::Value) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "{}".into())
}

/// Escape a field for CSV (quote and double internal quotes).
fn csv_escape(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}
