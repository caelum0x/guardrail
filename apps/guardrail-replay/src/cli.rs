//! Command-line surface for the event replay / audit tool.

use clap::{Parser, Subcommand};

pub const DEFAULT_DB_URL: &str = "sqlite://data/guardrail_alpha.db";
pub const DEFAULT_LIMIT: usize = 1000;

#[derive(Debug, Parser)]
#[command(name = "guardrail-replay", about = "Replay and audit the agent event log")]
pub struct Cli {
    /// sqlite:// database URL (or DATABASE_URL env).
    #[arg(long)]
    pub database_url: Option<String>,
    /// Maximum number of recent events to read.
    #[arg(long, default_value_t = DEFAULT_LIMIT)]
    pub limit: usize,
    /// Scope output to a single run id (exact match, or unique prefix).
    #[arg(long)]
    pub run: Option<String>,
    /// Emit machine-readable JSON instead of text, where the command supports it.
    #[arg(long, default_value_t = false)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print a chronological decision journal.
    Journal,
    /// Print confirmed on-chain swaps.
    Trades,
    /// Print risk rejections and clips with their reasons.
    Risk,
    /// Print event-type counts.
    Summary,
    /// Print the strategy → risk → execution funnel and lifecycle stats.
    Stats,
    /// List distinct runs with event counts and time spans.
    Runs,
    /// Export events as CSV to a path (or stdout with "-").
    ExportCsv { path: String },
}

impl Cli {
    /// Resolve the on-disk sqlite path from the flag, then `DATABASE_URL`, then
    /// the default, stripping any `sqlite://` scheme prefix.
    pub fn resolved_db_path(&self) -> String {
        let url = self
            .database_url
            .clone()
            .or_else(|| std::env::var("DATABASE_URL").ok())
            .unwrap_or_else(|| DEFAULT_DB_URL.into());
        url.strip_prefix("sqlite://").unwrap_or(&url).to_string()
    }
}
