//! Process and build metadata endpoint.
//!
//! Reports the crate version (from `CARGO_PKG_VERSION`), the build target, the
//! operating mode (from the `GUARDRAIL_MODE` environment variable, defaulting
//! to `paper`), and the process uptime in seconds since the first time this
//! endpoint was queried after start. Read-only and side-effect free; it never
//! panics and always returns a complete object.

use axum::Json;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Instant;

const DEFAULT_MODE: &str = "paper";

/// Captures the moment the process first observes the clock. Initialised on the
/// first call so uptime is measured from process start within the same run.
fn start_instant() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

/// Eagerly arm the uptime clock at process start so the first `/version` request
/// does not read a near-zero uptime. Safe to call multiple times.
pub fn init_uptime() {
    let _ = start_instant();
}

pub async fn version() -> Json<Value> {
    let uptime = start_instant().elapsed();
    let mode = std::env::var("GUARDRAIL_MODE").unwrap_or_else(|_| DEFAULT_MODE.to_string());

    Json(json!({
        "service": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "build_target": current_target(),
        "mode": mode,
        "uptime_seconds": uptime.as_secs(),
        "uptime_human": humanize(uptime.as_secs()),
    }))
}

/// The runtime build target, assembled from the standard architecture and OS
/// constants resolved at compile time.
fn current_target() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

/// Renders a whole-second duration as `Hh Mm Ss` for human-friendly display.
fn humanize(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{hours}h {minutes}m {seconds}s")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_formats_components() {
        assert_eq!(humanize(0), "0h 0m 0s");
        assert_eq!(humanize(3661), "1h 1m 1s");
        assert_eq!(humanize(59), "0h 0m 59s");
    }

    #[tokio::test]
    async fn version_returns_expected_shape() {
        init_uptime();
        let Json(value) = version().await;
        assert!(value["version"].is_string());
        assert!(value["uptime_seconds"].is_u64());
        assert_eq!(value["mode"], "paper");
    }
}
