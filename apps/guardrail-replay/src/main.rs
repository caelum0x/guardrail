//! Guardrail Alpha event replay / audit tool.
//!
//! Reads the agent's append-only SQLite event log and renders it for humans or
//! machines: a chronological decision journal, a confirmed-trade table, a risk
//! rejection/clip log, the strategy → risk → execution funnel, a per-run index,
//! or a CSV export. Read-only — it never trades and never mutates the log. This
//! is the audit trail that answers "why did it trade, what did it quote, what tx
//! resulted, and what did risk block?".

mod cli;
mod csv;
mod events;
mod json;
mod render;
mod stats;

use clap::Parser;
use event_store::SqliteEventRepository;

use crate::cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let path = cli.resolved_db_path();

    let repo = SqliteEventRepository::open(&path)
        .map_err(|e| anyhow::anyhow!("failed to open event log at {path}: {e}"))?;
    // `recent` returns newest-first; replay reads oldest-first.
    let mut events = repo.recent(cli.limit)?;
    events.reverse();
    let events = events::scope_to_run(events, cli.run.as_deref());

    // CSV always writes its own output regardless of `--json`.
    if let Command::ExportCsv { path } = &cli.command {
        return csv::export(&events, path);
    }

    if cli.json {
        if let Some(doc) = json::render(&cli.command, &events) {
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        return Ok(());
    }

    match cli.command {
        Command::Journal => render::journal(&events),
        Command::Trades => render::trades(&events),
        Command::Risk => render::risk(&events),
        Command::Summary => render::summary(&events),
        Command::Stats => render::stats(&events),
        Command::Runs => render::runs(&events),
        Command::ExportCsv { .. } => unreachable!("handled above"),
    }
    Ok(())
}
