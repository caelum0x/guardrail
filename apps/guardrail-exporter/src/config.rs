//! Exporter configuration, resolved from the environment.

use std::path::PathBuf;

pub const DEFAULT_DB_URL: &str = "sqlite://data/guardrail_alpha.db";
pub const DEFAULT_REPORT: &str = "data/run_report.json";
pub const DEFAULT_ADDR: &str = "0.0.0.0:9100";

#[derive(Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub report_path: String,
    pub addr: String,
}

impl Config {
    /// Build config from `DATABASE_URL`, `GUARDRAIL_REPORT`, and `EXPORTER_ADDR`,
    /// falling back to defaults. Strips the `sqlite://` scheme from the DB URL.
    pub fn from_env() -> Self {
        let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DB_URL.into());
        let db_path = PathBuf::from(db_url.strip_prefix("sqlite://").unwrap_or(&db_url));
        Config {
            db_path,
            report_path: std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into()),
            addr: std::env::var("EXPORTER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.into()),
        }
    }
}
