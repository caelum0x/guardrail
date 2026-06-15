//! Prometheus metrics exporter for Guardrail Alpha.
//!
//! A small, read-only sidecar that exposes `/metrics` in Prometheus text
//! exposition format. It derives gauges/counters from two sources the trading
//! agent already writes: the SQLite event log (event/trade/rejection/clip
//! counts, latest regime, last-event age) and `data/run_report.json` (NAV,
//! drawdown, positions, kill switch, report age).
//!
//! It never writes and never trades — purely observability.

mod config;
mod counts;
mod metrics;
mod report;

use axum::{extract::State, response::IntoResponse, routing::get, Router};

use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
        .ok();

    let config = Config::from_env();
    let addr = config.addr.clone();
    let app = Router::new()
        .route("/", get(index))
        .route("/metrics", get(metrics_handler))
        .route("/healthz", get(|| async { "ok" }))
        .with_state(config);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, "guardrail-exporter listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    (
        [("content-type", "text/plain; charset=utf-8")],
        "guardrail-exporter\n\nGET /metrics  Prometheus exposition\nGET /healthz  liveness\n",
    )
}

async fn metrics_handler(State(cfg): State<Config>) -> impl IntoResponse {
    let body = metrics::render(&cfg);
    ([("content-type", "text/plain; version=0.0.4")], body)
}
