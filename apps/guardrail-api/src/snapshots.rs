//! Read-only market-snapshot history endpoint.
//!
//! The agent persists one JSON `MarketSnapshot` per line to
//! `data/snapshots/<run_id>.jsonl` (override the base directory with
//! `GUARDRAIL_SNAPSHOT_DIR`). This endpoint surfaces that history:
//!
//! - Lists the available run files under the snapshot directory.
//! - For the most recent run (or the run named by `?run=<run_id>`) it returns a
//!   compact summary: `run_id`, cycle count (number of lines), first/last
//!   timestamps, and a small per-asset latest-price sample taken from the last
//!   line.
//!
//! It is strictly read-only and never panics on a missing directory or files:
//! a missing/empty directory yields an empty, typed result. Each line is parsed
//! independently as JSON; malformed lines are skipped rather than failing the
//! whole response.

use std::path::{Path, PathBuf};

use axum::{extract::Query, Json};
use serde::{Deserialize, Serialize};

/// Default snapshot directory, relative to the process working directory.
const DEFAULT_SNAPSHOT_DIR: &str = "data/snapshots";

/// Default number of per-asset price samples included in a run summary.
const DEFAULT_SAMPLE_LIMIT: usize = 8;

/// Maximum number of per-asset price samples a caller may request.
const MAX_SAMPLE_LIMIT: usize = 100;

/// Query parameters for `GET /snapshots`.
#[derive(Debug, Default, Deserialize)]
pub struct SnapshotsParams {
    /// Optional explicit run id to summarize instead of the most recent run.
    pub run: Option<String>,
    /// Optional cap on the number of per-asset price samples returned.
    pub limit: Option<usize>,
}

/// One discovered run file in the snapshot directory.
#[derive(Debug, Clone, Serialize)]
pub struct RunFile {
    /// Run id, derived from the file stem (`<run_id>.jsonl`).
    pub run_id: String,
    /// Last-modified time in milliseconds since the Unix epoch, when available.
    pub modified_ms: Option<i64>,
}

/// A single per-asset latest-price sample drawn from the last snapshot line.
#[derive(Debug, Clone, Serialize)]
pub struct PriceSample {
    pub symbol: String,
    pub price_usd: String,
}

/// Compact summary of a single run's snapshot history.
#[derive(Debug, Clone, Serialize)]
pub struct RunSummary {
    pub run_id: String,
    /// Number of well-formed snapshot lines (cycles) in the file.
    pub cycle_count: usize,
    /// Number of lines that could not be parsed and were skipped.
    pub skipped_lines: usize,
    /// Timestamp (ms) of the first parsed snapshot, if any.
    pub first_timestamp_ms: Option<i64>,
    /// Timestamp (ms) of the last parsed snapshot, if any.
    pub last_timestamp_ms: Option<i64>,
    /// Per-asset latest-price sample from the last parsed line.
    pub latest_prices: Vec<PriceSample>,
}

/// Top-level response for `GET /snapshots`.
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotsResponse {
    /// Resolved snapshot directory that was inspected.
    pub directory: String,
    /// All discovered run files, newest first.
    pub runs: Vec<RunFile>,
    /// Summary of the selected run (most recent by default), if one exists.
    pub latest: Option<RunSummary>,
}

/// Resolve the snapshot directory from the environment, falling back to the
/// default. Mirrors the agent's `persist_snapshot` resolution.
fn snapshot_dir() -> PathBuf {
    std::env::var("GUARDRAIL_SNAPSHOT_DIR")
        .unwrap_or_else(|_| DEFAULT_SNAPSHOT_DIR.to_string())
        .into()
}

/// `GET /snapshots` — list runs and summarize the selected (default: most
/// recent) run. Read-only and infallible: errors degrade to an empty result.
pub async fn snapshots(Query(params): Query<SnapshotsParams>) -> Json<SnapshotsResponse> {
    Json(build_response(&snapshot_dir(), &params))
}

/// Pure core of the handler, factored out so it can be exercised over a temp
/// directory in tests without binding a socket.
fn build_response(dir: &Path, params: &SnapshotsParams) -> SnapshotsResponse {
    let runs = list_runs(dir);
    let sample_limit = params
        .limit
        .unwrap_or(DEFAULT_SAMPLE_LIMIT)
        .min(MAX_SAMPLE_LIMIT);

    // Select the requested run if given, otherwise the most recent one.
    let selected = match &params.run {
        Some(requested) => runs.iter().find(|r| &r.run_id == requested),
        None => runs.first(),
    };

    let latest = selected.map(|run| summarize_run(dir, &run.run_id, sample_limit));

    SnapshotsResponse {
        directory: dir.to_string_lossy().into_owned(),
        runs,
        latest,
    }
}

/// Enumerate `*.jsonl` files in `dir`, newest first by modification time.
/// Returns an empty vector when the directory is missing or unreadable.
fn list_runs(dir: &Path) -> Vec<RunFile> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut runs: Vec<(RunFile, Option<std::time::SystemTime>)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(run_id) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
        runs.push((
            RunFile {
                run_id: run_id.to_string(),
                modified_ms: modified.and_then(system_time_to_ms),
            },
            modified,
        ));
    }

    // Newest first; entries without a modification time sort last, then by id
    // for a stable, deterministic order.
    runs.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.run_id.cmp(&b.0.run_id))
    });

    runs.into_iter().map(|(run, _)| run).collect()
}

/// Read and summarize a single run file. Never panics: an unreadable file
/// yields an empty summary; malformed lines are counted and skipped.
fn summarize_run(dir: &Path, run_id: &str, sample_limit: usize) -> RunSummary {
    let path = dir.join(format!("{run_id}.jsonl"));
    let contents = std::fs::read_to_string(&path).unwrap_or_default();

    let mut cycle_count = 0usize;
    let mut skipped_lines = 0usize;
    let mut first_timestamp_ms: Option<i64> = None;
    let mut last_timestamp_ms: Option<i64> = None;
    let mut last_value: Option<serde_json::Value> = None;

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => {
                cycle_count += 1;
                let ts = value.get("timestamp_ms").and_then(serde_json::Value::as_i64);
                if first_timestamp_ms.is_none() {
                    first_timestamp_ms = ts;
                }
                if ts.is_some() {
                    last_timestamp_ms = ts;
                }
                last_value = Some(value);
            }
            Err(_) => skipped_lines += 1,
        }
    }

    let latest_prices = last_value
        .as_ref()
        .map(|value| price_samples(value, sample_limit))
        .unwrap_or_default();

    RunSummary {
        run_id: run_id.to_string(),
        cycle_count,
        skipped_lines,
        first_timestamp_ms,
        last_timestamp_ms,
        latest_prices,
    }
}

/// Extract up to `limit` per-asset `(symbol, price_usd)` samples from a parsed
/// snapshot value. Tolerant of missing fields and varying value shapes.
fn price_samples(snapshot: &serde_json::Value, limit: usize) -> Vec<PriceSample> {
    let Some(assets) = snapshot.get("assets").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };

    assets
        .iter()
        .filter_map(|asset| {
            let symbol = asset
                .get("asset")
                .and_then(|a| a.get("symbol"))
                .and_then(serde_json::Value::as_str)?;
            let price_usd = value_as_string(asset.get("price_usd")?)?;
            Some(PriceSample {
                symbol: symbol.to_string(),
                price_usd,
            })
        })
        .take(limit)
        .collect()
}

/// Render a JSON value as a price string. `price_usd` is serialized as a string
/// by the agent (rust_decimal), but we also accept raw numbers defensively.
fn value_as_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Convert a `SystemTime` to milliseconds since the Unix epoch, if it is at or
/// after the epoch.
fn system_time_to_ms(time: std::time::SystemTime) -> Option<i64> {
    time.duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a `.jsonl` file under `dir` and return nothing; helper for tests.
    fn write_run(dir: &Path, run_id: &str, lines: &[&str]) {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(format!("{run_id}.jsonl"));
        std::fs::write(path, lines.join("\n")).unwrap();
    }

    fn snapshot_line(ts: i64, prices: &[(&str, &str)]) -> String {
        let assets: Vec<serde_json::Value> = prices
            .iter()
            .map(|(sym, price)| {
                serde_json::json!({
                    "asset": { "symbol": sym },
                    "price_usd": price,
                })
            })
            .collect();
        serde_json::json!({ "timestamp_ms": ts, "assets": assets }).to_string()
    }

    #[test]
    fn missing_directory_yields_empty_typed_result() {
        let dir = std::env::temp_dir().join("guardrail-snap-missing-xyz-123");
        let _ = std::fs::remove_dir_all(&dir);
        let resp = build_response(&dir, &SnapshotsParams::default());
        assert!(resp.runs.is_empty());
        assert!(resp.latest.is_none());
    }

    #[test]
    fn summarizes_most_recent_run_with_price_sample() {
        let base = std::env::temp_dir().join(format!("guardrail-snap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        write_run(
            &base,
            "run_a",
            &[
                &snapshot_line(1000, &[("BTC", "60000")]),
                &snapshot_line(2000, &[("BTC", "61000"), ("ETH", "3000")]),
            ],
        );

        let resp = build_response(&base, &SnapshotsParams::default());
        assert_eq!(resp.runs.len(), 1);
        let latest = resp.latest.expect("summary present");
        assert_eq!(latest.run_id, "run_a");
        assert_eq!(latest.cycle_count, 2);
        assert_eq!(latest.skipped_lines, 0);
        assert_eq!(latest.first_timestamp_ms, Some(1000));
        assert_eq!(latest.last_timestamp_ms, Some(2000));
        // Latest line carried two assets.
        assert_eq!(latest.latest_prices.len(), 2);
        assert_eq!(latest.latest_prices[0].symbol, "BTC");
        assert_eq!(latest.latest_prices[0].price_usd, "61000");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn skips_malformed_lines_without_panicking() {
        let base =
            std::env::temp_dir().join(format!("guardrail-snap-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        write_run(
            &base,
            "run_bad",
            &[
                &snapshot_line(10, &[("BTC", "1")]),
                "{ this is not json",
                "",
                &snapshot_line(20, &[("BTC", "2")]),
            ],
        );

        let summary = summarize_run(&base, "run_bad", DEFAULT_SAMPLE_LIMIT);
        assert_eq!(summary.cycle_count, 2);
        assert_eq!(summary.skipped_lines, 1);
        assert_eq!(summary.last_timestamp_ms, Some(20));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn honors_explicit_run_and_sample_limit() {
        let base =
            std::env::temp_dir().join(format!("guardrail-snap-sel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);

        write_run(&base, "run_one", &[&snapshot_line(1, &[("BTC", "1")])]);
        write_run(
            &base,
            "run_two",
            &[&snapshot_line(2, &[("A", "1"), ("B", "2"), ("C", "3")])],
        );

        let params = SnapshotsParams {
            run: Some("run_two".to_string()),
            limit: Some(2),
        };
        let resp = build_response(&base, &params);
        let latest = resp.latest.expect("summary present");
        assert_eq!(latest.run_id, "run_two");
        assert_eq!(latest.latest_prices.len(), 2);

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Live HTTP check: bind the `/snapshots` route on an ephemeral port,
    /// serve it, and assert the handler returns `200 OK` with a well-formed
    /// JSON body over the wire. This exercises the same wiring used in
    /// `server::build_app` without contending for the production `:8080`.
    #[tokio::test]
    async fn route_serves_200_with_json_body_over_http() {
        use axum::{routing::get, Router};

        let base =
            std::env::temp_dir().join(format!("guardrail-snap-http-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        write_run(
            &base,
            "run_http",
            &[&snapshot_line(42, &[("BTC", "60000"), ("ETH", "3000")])],
        );
        // The handler resolves the directory from the environment.
        std::env::set_var("GUARDRAIL_SNAPSHOT_DIR", &base);

        let app = Router::new().route("/snapshots", get(snapshots));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("http://{addr}/snapshots?run=run_http&limit=1");
        let (status, body) = http_get(&addr.to_string(), "/snapshots?run=run_http&limit=1").await;
        assert_eq!(status, 200, "unexpected status for {url}: body={body}");

        let value: serde_json::Value = serde_json::from_str(&body).expect("valid JSON body");
        assert_eq!(value["latest"]["run_id"], "run_http");
        assert_eq!(value["latest"]["cycle_count"], 1);
        assert_eq!(value["latest"]["last_timestamp_ms"], 42);
        // limit=1 caps the per-asset price sample.
        assert_eq!(value["latest"]["latest_prices"].as_array().unwrap().len(), 1);

        server.abort();
        std::env::remove_var("GUARDRAIL_SNAPSHOT_DIR");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Minimal HTTP/1.1 GET client so the live test needs no extra dependency.
    /// Returns `(status_code, body)`.
    async fn http_get(addr: &str, path: &str) -> (u16, String) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        let text = String::from_utf8_lossy(&raw).into_owned();

        let status = text
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .expect("status code in response line");
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default();
        (status, body)
    }
}
