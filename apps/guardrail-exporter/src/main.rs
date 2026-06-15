//! Prometheus metrics exporter for Guardrail Alpha.
//!
//! A small, read-only sidecar that exposes `/metrics` in Prometheus text
//! exposition format. It derives gauges/counters from two sources the trading
//! agent already writes: the SQLite event log (event/trade/rejection counts)
//! and `data/run_report.json` (NAV, drawdown, positions, kill switch, age).
//!
//! It never writes and never trades — purely observability.

use axum::{extract::State, response::IntoResponse, routing::get, Router};
use event_store::{AgentEvent, SqliteEventRepository, StoredEvent};
use std::path::PathBuf;

const DEFAULT_DB_URL: &str = "sqlite://data/guardrail_alpha.db";
const DEFAULT_REPORT: &str = "data/run_report.json";
const DEFAULT_ADDR: &str = "0.0.0.0:9100";
const SCAN_LIMIT: usize = 10_000;

#[derive(Clone)]
struct Config {
    db_path: PathBuf,
    report_path: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
        .ok();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DB_URL.into());
    let db_path = PathBuf::from(db_url.strip_prefix("sqlite://").unwrap_or(&db_url));
    let report_path = std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT.into());
    let addr = std::env::var("EXPORTER_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.into());

    let config = Config {
        db_path,
        report_path,
    };
    let app = Router::new()
        .route("/metrics", get(metrics))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(config);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "guardrail-exporter listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn metrics(State(cfg): State<Config>) -> impl IntoResponse {
    let body = render(&cfg);
    ([("content-type", "text/plain; version=0.0.4")], body)
}

/// Build the full Prometheus exposition body.
fn render(cfg: &Config) -> String {
    let mut out = String::new();
    let counts = event_counts(cfg);
    let report = load_report(cfg);

    gauge(
        &mut out,
        "guardrail_events_total",
        "Total recorded agent events",
        counts.events as f64,
    );
    gauge(
        &mut out,
        "guardrail_trades_total",
        "Confirmed on-chain swaps",
        counts.trades as f64,
    );
    gauge(
        &mut out,
        "guardrail_risk_rejections_total",
        "Orders rejected by the risk engine",
        counts.rejections as f64,
    );
    gauge(
        &mut out,
        "guardrail_orders_proposed_total",
        "Orders proposed by the strategy",
        counts.proposed as f64,
    );
    gauge(
        &mut out,
        "guardrail_quotes_total",
        "TWAK quotes received",
        counts.quotes as f64,
    );
    gauge(
        &mut out,
        "guardrail_daily_trade_satisfied_total",
        "Cycles satisfying the daily-trade requirement",
        counts.daily_satisfied as f64,
    );

    if let Some(r) = report {
        if let Some(v) = num(&r, "nav_usd") {
            gauge(&mut out, "guardrail_nav_usd", "Net asset value in USD", v);
        }
        if let Some(v) = num(&r, "total_drawdown_pct") {
            gauge(
                &mut out,
                "guardrail_total_drawdown_pct",
                "Total drawdown percent",
                v,
            );
        }
        let position_arr = r.get("positions").and_then(|p| p.as_array());
        let positions = position_arr.map(|a| a.len()).unwrap_or(0);
        gauge(
            &mut out,
            "guardrail_positions",
            "Open non-reserve positions",
            positions as f64,
        );
        // Per-asset weight gauges (labeled by symbol).
        if let Some(arr) = position_arr {
            out.push_str(
                "# HELP guardrail_position_weight_pct Position weight as percent of NAV\n",
            );
            out.push_str("# TYPE guardrail_position_weight_pct gauge\n");
            for p in arr {
                let symbol = p.get("symbol").and_then(|v| v.as_str()).unwrap_or("");
                let weight = p
                    .get("weight_pct")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                if !symbol.is_empty() {
                    out.push_str(&format!(
                        "guardrail_position_weight_pct{{symbol=\"{symbol}\"}} {weight}\n"
                    ));
                }
            }
        }
        let kill = r
            .get("kill_switch")
            .and_then(|k| k.as_bool())
            .unwrap_or(false);
        gauge(
            &mut out,
            "guardrail_kill_switch",
            "Kill switch engaged (1) or armed (0)",
            if kill { 1.0 } else { 0.0 },
        );
        if let Some(updated_ms) = r.get("updated_ms").and_then(|v| v.as_i64()) {
            let age = ((now_ms() - updated_ms).max(0)) as f64 / 1000.0;
            gauge(
                &mut out,
                "guardrail_report_age_seconds",
                "Seconds since the last run report",
                age,
            );
        }
    }

    out
}

#[derive(Default)]
struct Counts {
    events: usize,
    trades: usize,
    rejections: usize,
    proposed: usize,
    quotes: usize,
    daily_satisfied: usize,
}

/// Count events by type from the SQLite log. Returns zeros on any read error.
fn event_counts(cfg: &Config) -> Counts {
    let events: Vec<StoredEvent> = SqliteEventRepository::open(&cfg.db_path)
        .and_then(|repo| repo.recent(SCAN_LIMIT))
        .unwrap_or_default();
    let mut c = Counts {
        events: events.len(),
        ..Default::default()
    };
    for e in &events {
        match e.event_type {
            AgentEvent::TxConfirmed => c.trades += 1,
            AgentEvent::RiskRejected => c.rejections += 1,
            AgentEvent::OrderProposed => c.proposed += 1,
            AgentEvent::TwakQuoteReceived => c.quotes += 1,
            AgentEvent::DailyTradeRequirementSatisfied => c.daily_satisfied += 1,
            _ => {}
        }
    }
    c
}

/// Load and parse the run report JSON, if present.
fn load_report(cfg: &Config) -> Option<serde_json::Value> {
    let raw = std::fs::read_to_string(&cfg.report_path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Parse a string-or-number JSON field into f64.
fn num(report: &serde_json::Value, key: &str) -> Option<f64> {
    match report.get(key) {
        Some(serde_json::Value::String(s)) => s.parse().ok(),
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Append one gauge metric (HELP + TYPE + sample) to the buffer.
fn gauge(out: &mut String, name: &str, help: &str, value: f64) {
    out.push_str(&format!("# HELP {name} {help}\n"));
    out.push_str(&format!("# TYPE {name} gauge\n"));
    out.push_str(&format!("{name} {value}\n"));
}
