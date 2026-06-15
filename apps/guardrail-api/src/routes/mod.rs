use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{extract::State, Json};
use event_store::{AgentEvent, SqliteEventRepository, StoredEvent};
use observability::{evaluate_all, Alert, AlertInputs, AlertKind, AlertThresholds, Severity};
use serde_json::{json, Value};
use std::path::PathBuf;

const DEFAULT_DATABASE_URL: &str = "sqlite://data/guardrail_alpha.db";
const DEFAULT_REPORT_PATH: &str = "data/run_report.json";
const PRODUCTION_POLICY_PATH: &str = "configs/risk_policy.production.json";
const PAPER_POLICY_PATH: &str = "configs/risk_policy.paper.json";
const UNIVERSE_PATH: &str = "configs/eligible_assets.bsc.json";
const PAPER_CONFIG_PATH: &str = "configs/paper.toml";
const PRODUCTION_CONFIG_PATH: &str = "configs/production.toml";
const BACKTEST_CONFIG_PATH: &str = "configs/backtest.toml";
const RECENT_LIMIT: usize = 200;

#[derive(Debug, Clone)]
pub struct AppState {
    database_url: String,
    report_path: PathBuf,
}

impl AppState {
    pub fn new(database_url: impl Into<String>) -> Self {
        Self {
            database_url: database_url.into(),
            report_path: report_path_from_env(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into()))
    }

    fn db_path(&self) -> anyhow::Result<PathBuf> {
        sqlite_path(&self.database_url)
            .ok_or_else(|| anyhow::anyhow!("DATABASE_URL must start with sqlite://"))
    }

    fn repo(&self) -> anyhow::Result<SqliteEventRepository> {
        SqliteEventRepository::open(self.db_path()?)
    }

    /// Read the most recent stored events (newest-first), surfacing any error.
    ///
    /// Public read-only accessor used by sibling endpoint modules that need
    /// raw event access without depending on this module's private helpers.
    pub fn recent_events(&self, limit: usize) -> anyhow::Result<Vec<StoredEvent>> {
        self.repo()?.recent(limit)
    }
}

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    match state.repo() {
        Ok(repo) => Json(json!({
            "ok": true,
            "database_url": state.database_url,
            "events_visible": repo.recent(1).map(|events| events.len()).unwrap_or(0)
        })),
        Err(error) => Json(json!({
            "ok": false,
            "database_url": state.database_url,
            "error": error.to_string()
        })),
    }
}

pub async fn portfolio(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let latest = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::PortfolioReconciled)
    });
    Json(json!({
        "latest": latest.map(|event| &event.payload_json),
        "source_event_id": latest.map(|event| &event.id)
    }))
}

pub async fn trades(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let trades: Vec<&StoredEvent> = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                AgentEvent::OrderProposed
                    | AgentEvent::TwakQuoteReceived
                    | AgentEvent::TwakSwapSubmitted
                    | AgentEvent::TxConfirmed
            )
        })
        .collect();
    Json(json!({ "trades": trades }))
}

pub async fn signals(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let regime = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::RegimeClassified)
    });
    let target = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::PortfolioTargetComputed)
    });
    Json(json!({
        "regime": regime.map(|event| &event.payload_json),
        "target": target.map(|event| &event.payload_json)
    }))
}

pub async fn risk(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let risk_events: Vec<&StoredEvent> = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                AgentEvent::RiskApproved
                    | AgentEvent::RiskRejected
                    | AgentEvent::RiskClipped
                    | AgentEvent::DrawdownThrottleActivated
                    | AgentEvent::KillSwitchTriggered
            )
        })
        .collect();
    let kill_switch = risk_events
        .iter()
        .any(|event| matches!(event.event_type, AgentEvent::KillSwitchTriggered));
    Json(json!({ "kill_switch": kill_switch, "events": risk_events }))
}

pub async fn alerts(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let report_result = read_run_report(&state);
    let mut report_alerts = Vec::new();
    let report = match report_result {
        Ok(report) => report,
        Err(error) => {
            report_alerts.push(Alert::new(
                AlertKind::DataStale,
                Severity::Critical,
                format!("run report unavailable: {error}"),
            ));
            json!({})
        }
    };
    let thresholds = alert_thresholds();
    let inputs = alert_inputs(&report, &events);
    let mut alerts = evaluate_all(&inputs, &thresholds);
    alerts.extend(report_alerts);
    alerts.sort_by_key(|alert| match alert.severity {
        Severity::Critical => 0,
        Severity::Warning => 1,
        Severity::Info => 2,
    });
    let critical = alerts
        .iter()
        .filter(|alert| matches!(alert.severity, Severity::Critical))
        .count();
    let warning = alerts
        .iter()
        .filter(|alert| matches!(alert.severity, Severity::Warning))
        .count();
    let status = if critical > 0 {
        "critical"
    } else if warning > 0 {
        "warning"
    } else {
        "clear"
    };
    let trades_visible = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .count();

    Json(json!({
        "status": status,
        "counts": {
            "critical": critical,
            "warning": warning,
            "total": alerts.len()
        },
        "alerts": alerts,
        "inputs": {
            "report_age_seconds": inputs.data_age_secs,
            "total_drawdown_pct": metric_value(&report, "total_drawdown_pct"),
            "drawdown_soft_limit_pct": thresholds.drawdown_soft * 100.0,
            "drawdown_hard_limit_pct": thresholds.drawdown_hard * 100.0,
            "latest_slippage_pct": inputs.slippage * 100.0,
            "slippage_limit_pct": thresholds.slippage_max * 100.0,
            "kill_switch": inputs.kill_switch,
            "daily_trade_executed": inputs.daily_trade_executed,
            "events_visible": events.len(),
            "trades_visible": trades_visible,
            "report_path": state.report_path.display().to_string()
        }
    }))
}

pub async fn readiness(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let report = read_run_report(&state).ok();
    let empty_report = json!({});
    let alerts = active_alerts(report.as_ref().unwrap_or(&empty_report), &events);
    let critical_alerts = alerts
        .iter()
        .filter(|alert| matches!(alert.severity, Severity::Critical))
        .count();
    let market_snapshots = event_count(&events, AgentEvent::MarketSnapshotReceived);
    let risk_decisions = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                AgentEvent::RiskApproved | AgentEvent::RiskRejected | AgentEvent::RiskClipped
            )
        })
        .count();
    let quotes = event_count(&events, AgentEvent::TwakQuoteReceived);
    let confirmed_txs = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .count();
    let daily_trade = events.iter().any(|event| {
        matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied)
            || (matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some())
    });
    let wallet = report
        .as_ref()
        .map(|value| text(value, "wallet_address"))
        .unwrap_or("pending");
    let policy_hash = report
        .as_ref()
        .map(|value| text(value, "policy_hash"))
        .unwrap_or("pending");

    let checks = vec![
        readiness_check(
            "run_report",
            "Run report generated",
            report.is_some(),
            format!("path {}", state.report_path.display()),
        ),
        readiness_check(
            "wallet",
            "Agent wallet present",
            wallet != "pending" && !wallet.is_empty(),
            wallet.to_string(),
        ),
        readiness_check(
            "policy_hash",
            "Policy hash present",
            policy_hash != "pending" && !policy_hash.is_empty(),
            policy_hash.to_string(),
        ),
        readiness_check(
            "events",
            "Event log populated",
            !events.is_empty(),
            format!("{} events visible", events.len()),
        ),
        readiness_check(
            "market_data",
            "Market data captured",
            market_snapshots > 0,
            format!("{market_snapshots} market snapshots"),
        ),
        readiness_check(
            "risk_decisions",
            "Risk gate exercised",
            risk_decisions > 0,
            format!("{risk_decisions} risk decisions"),
        ),
        readiness_check(
            "twak_quotes",
            "TWAK quote evidence",
            quotes > 0,
            format!("{quotes} quotes"),
        ),
        readiness_check(
            "confirmed_txs",
            "Transaction proof captured",
            confirmed_txs > 0,
            format!("{confirmed_txs} confirmed transactions"),
        ),
        readiness_check(
            "daily_trade",
            "Daily trade requirement satisfied",
            daily_trade,
            if daily_trade { "satisfied" } else { "missing" }.to_string(),
        ),
        readiness_check(
            "critical_alerts",
            "No critical operator alerts",
            critical_alerts == 0,
            format!("{critical_alerts} critical alerts"),
        ),
    ];
    let blocking = checks
        .iter()
        .filter(|check| check.get("status").and_then(Value::as_str) == Some("blocking"))
        .count();
    let status = if blocking > 0 { "blocking" } else { "ready" };

    Json(json!({
        "status": status,
        "blocking": blocking,
        "checks": checks,
        "artifacts": {
            "report": "/report",
            "submission_markdown": "/export/submission.md",
            "proof": "/proof",
            "alerts": "/alerts"
        }
    }))
}

pub async fn events(State(state): State<AppState>) -> Json<Value> {
    match state.repo().and_then(|repo| repo.recent(RECENT_LIMIT)) {
        Ok(events) => Json(json!({ "events": events })),
        Err(error) => Json(json!({ "events": [], "error": error.to_string() })),
    }
}

pub async fn proof(State(state): State<AppState>) -> Json<Value> {
    let events = read_recent(&state);
    let run_report = read_run_report(&state).ok();
    let competition_tx = events.iter().find_map(|event| {
        if matches!(event.event_type, AgentEvent::TxConfirmed) {
            event
                .payload_json
                .get("competition_tx")
                .and_then(|value| value.as_str())
        } else {
            None
        }
    });
    let latest_report = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::AgentReportPublished)
    });

    Json(json!({
        "agent": "guardrail-alpha",
        "registration_tx": competition_tx,
        "latest_report": latest_report.map(|event| &event.payload_json),
        "run_report": run_report,
        "source_event_id": latest_report.map(|event| &event.id)
    }))
}

pub async fn cockpit(State(state): State<AppState>) -> Json<Value> {
    let health = match state.repo() {
        Ok(repo) => json!({
            "ok": true,
            "database_url": state.database_url,
            "events_visible": repo.recent(1).map(|events| events.len()).unwrap_or(0)
        }),
        Err(error) => json!({
            "ok": false,
            "database_url": state.database_url,
            "error": error.to_string()
        }),
    };
    let events = read_recent(&state);
    let run_report = read_run_report(&state).ok();
    let latest_report = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::AgentReportPublished)
    });
    let portfolio = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::PortfolioReconciled)
    });
    let regime = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::RegimeClassified)
    });
    let target = latest_event(&events, |event| {
        matches!(event.event_type, AgentEvent::PortfolioTargetComputed)
    });
    let kill_switch = events
        .iter()
        .any(|event| matches!(event.event_type, AgentEvent::KillSwitchTriggered));
    let tx_count = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .count();
    let risk_count = events
        .iter()
        .filter(|event| {
            matches!(
                event.event_type,
                AgentEvent::RiskApproved | AgentEvent::RiskRejected | AgentEvent::RiskClipped
            )
        })
        .count();
    let latest_tx = events.iter().find(|event| {
        matches!(event.event_type, AgentEvent::TxConfirmed)
            && event.payload_json.get("tx_hash").is_some()
    });
    let recent_activity: Vec<&StoredEvent> = events.iter().take(24).collect();

    Json(json!({
        "health": health,
        "latest_report": latest_report.map(|event| &event.payload_json),
        "run_report": run_report,
        "portfolio": portfolio.map(|event| &event.payload_json),
        "regime": regime.map(|event| &event.payload_json),
        "target": target.map(|event| &event.payload_json),
        "risk": {
            "kill_switch": kill_switch,
            "recent_decisions": risk_count
        },
        "execution": {
            "confirmed_txs": tx_count,
            "latest_tx": latest_tx.map(|event| &event.payload_json)
        },
        "activity": recent_activity
    }))
}

pub async fn report_json(State(state): State<AppState>) -> Json<Value> {
    match read_run_report(&state) {
        Ok(report) => Json(json!({ "ok": true, "report": report })),
        Err(error) => Json(json!({
            "ok": false,
            "path": state.report_path.display().to_string(),
            "error": error.to_string()
        })),
    }
}

pub async fn report_markdown(State(state): State<AppState>) -> Response {
    match read_run_report(&state) {
        Ok(report) => markdown_response(render_run_report_markdown(&report), "run_report.md"),
        Err(error) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("run report unavailable: {error}"),
        )
            .into_response(),
    }
}

pub async fn submission_markdown(State(state): State<AppState>) -> Response {
    let events = read_recent(&state);
    match read_run_report(&state) {
        Ok(report) => {
            let markdown = render_submission_markdown(&report, &events);
            markdown_response(markdown, "guardrail-alpha-submission.md")
        }
        Err(error) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            format!("submission export unavailable: {error}"),
        )
            .into_response(),
    }
}

pub async fn policy() -> Json<Value> {
    Json(json!({
        "production": file_result(PRODUCTION_POLICY_PATH, read_json_file(PRODUCTION_POLICY_PATH)),
        "paper": file_result(PAPER_POLICY_PATH, read_json_file(PAPER_POLICY_PATH)),
        "schema": file_result("configs/risk_policy.schema.json", read_json_file("configs/risk_policy.schema.json")),
        "enforcement": {
            "execution_layer": "twak_only",
            "quote_before_swap": true,
            "risk_gate": "No RiskDecision::Approved means no TWAK swap",
            "api_mode": "read_only"
        }
    }))
}

pub async fn universe() -> Json<Value> {
    let raw = read_json_file(UNIVERSE_PATH);
    let enabled_assets = raw
        .as_ref()
        .ok()
        .and_then(Value::as_array)
        .map(|assets| {
            assets
                .iter()
                .filter(|asset| {
                    asset
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);

    Json(json!({
        "path": UNIVERSE_PATH,
        "enabled_assets": enabled_assets,
        "assets": raw.unwrap_or_else(|error| json!({ "error": error.to_string() }))
    }))
}

pub async fn config_inventory() -> Json<Value> {
    Json(json!({
        "runtime": {
            "paper": read_text_file(PAPER_CONFIG_PATH).unwrap_or_default(),
            "production": read_text_file(PRODUCTION_CONFIG_PATH).unwrap_or_default(),
            "backtest": read_text_file(BACKTEST_CONFIG_PATH).unwrap_or_default()
        },
        "strategy_weights": file_result("configs/strategy_weights.json", read_json_file("configs/strategy_weights.json")),
        "execution_limits": file_result("configs/execution_limits.json", read_json_file("configs/execution_limits.json")),
        "asset_categories": file_result("configs/asset_categories.json", read_json_file("configs/asset_categories.json")),
        "secrets_template": "configs/secrets.example.toml",
        "environment": {
            "database_url": std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string()),
            "report_path": std::env::var("GUARDRAIL_REPORT").unwrap_or_else(|_| DEFAULT_REPORT_PATH.to_string())
        }
    }))
}

pub async fn ops() -> Json<Value> {
    Json(json!({
        "mode": "read_only_api",
        "operator_commands": [
            { "name": "paper agent", "command": "GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml" },
            { "name": "api", "command": "DATABASE_URL=sqlite://data/guardrail_alpha.db cargo run -p guardrail-api" },
            { "name": "dashboard", "command": "cd dashboard && pnpm dev" },
            { "name": "monitor", "command": "GUARDRAIL_MONITOR_CHECKS=0 cargo run -p guardrail-monitor" },
            { "name": "backtest", "command": "cargo run -p guardrail-cli -- backtest --config configs/backtest.toml" },
            { "name": "score", "command": "cargo run -p guardrail-cli -- score --config configs/paper.toml" },
            { "name": "readiness", "command": "./scripts/readiness.sh" },
            { "name": "export", "command": "./scripts/export_report.sh" },
            { "name": "kill switch", "command": "./scripts/kill_switch.sh" }
        ],
        "http_surfaces": [
            "/health",
            "/cockpit",
            "/alerts",
            "/audit-manifest",
            "/agent-card",
            "/agent-services",
            "/bnb-sdk",
            "/readiness",
            "/events",
            "/exposure",
            "/metrics",
            "/report",
            "/report/markdown",
            "/export/submission.md",
            "/policy",
            "/universe",
            "/config",
            "/commerce",
            "/costs",
            "/briefing",
            "/budget",
            "/heartbeat",
            "/job-simulator",
            "/drift",
            "/exit-triggers",
            "/ops",
            "/playbook",
            "/prizes",
            "/quotes",
            "/watchlist",
            "/wallet-controls",
            "/liquidity",
            "/mandates",
            "/regime",
            "/funding",
            "/scenarios",
            "/rebalance",
            "/scorecard",
            "/sdk-catalog",
            "/signing-policy"
        ],
        "docker": {
            "compose": "docker compose up --build",
            "api": "infra/Dockerfile.api",
            "agent": "infra/Dockerfile.agent",
            "dashboard": "infra/Dockerfile.dashboard",
            "monitor": "infra/Dockerfile.monitor"
        },
        "safety": [
            "API is read-only",
            "Dashboard cannot call TWAK",
            "Execution path requires risk approval",
            "Monitor reads reports and alerts only"
        ]
    }))
}

pub async fn metrics(State(state): State<AppState>) -> Response {
    let events = read_recent(&state);
    let report = read_run_report(&state).unwrap_or_else(|_| json!({}));
    let body = render_prometheus_metrics(&report, &events);
    (
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

fn read_recent(state: &AppState) -> Vec<StoredEvent> {
    state
        .repo()
        .and_then(|repo| repo.recent(RECENT_LIMIT))
        .unwrap_or_default()
}

fn read_json_file(path: &str) -> anyhow::Result<Value> {
    Ok(serde_json::from_str(&read_text_file(path)?)?)
}

fn read_text_file(path: &str) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(path)?)
}

fn file_result(path: &str, value: anyhow::Result<Value>) -> Value {
    match value {
        Ok(value) => json!({ "path": path, "ok": true, "value": value }),
        Err(error) => json!({ "path": path, "ok": false, "error": error.to_string() }),
    }
}

fn read_run_report(state: &AppState) -> anyhow::Result<Value> {
    let raw = std::fs::read_to_string(&state.report_path).map_err(|e| {
        anyhow::anyhow!(
            "failed to read run report {}: {e}",
            state.report_path.display()
        )
    })?;
    Ok(serde_json::from_str(&raw)?)
}

fn report_path_from_env() -> PathBuf {
    std::env::var("GUARDRAIL_REPORT")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REPORT_PATH))
}

fn markdown_response(markdown: String, filename: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("inline; filename=\"{filename}\""),
            ),
        ],
        markdown,
    )
        .into_response()
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("pending")
}

fn number(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|v| v.to_string())
        .or_else(|| {
            value
                .get(key)
                .and_then(Value::as_u64)
                .map(|v| v.to_string())
        })
        .unwrap_or_else(|| "0".to_string())
}

fn render_run_report_markdown(report: &Value) -> String {
    let mut out = String::new();
    out.push_str("# Guardrail Alpha Run Report\n\n");
    out.push_str("| Field | Value |\n| --- | --- |\n");
    out.push_str(&format!("| Run id | `{}` |\n", text(report, "run_id")));
    out.push_str(&format!("| Mode | {} |\n", text(report, "mode")));
    out.push_str(&format!(
        "| Wallet | `{}` |\n",
        text(report, "wallet_address")
    ));
    out.push_str(&format!("| NAV | ${} |\n", text(report, "nav_usd")));
    out.push_str(&format!(
        "| Total drawdown | {}% |\n",
        text(report, "total_drawdown_pct")
    ));
    out.push_str(&format!("| Regime | {} |\n", text(report, "regime")));
    out.push_str(&format!("| Events | {} |\n", number(report, "events")));
    out.push_str(&format!(
        "| Policy hash | `{}` |\n",
        text(report, "policy_hash")
    ));
    out.push_str("\n## Positions\n\n| Symbol | Weight | Value |\n| --- | ---: | ---: |\n");
    if let Some(positions) = report.get("positions").and_then(Value::as_array) {
        for p in positions {
            out.push_str(&format!(
                "| {} | {}% | ${} |\n",
                text(p, "symbol"),
                text(p, "weight_pct"),
                text(p, "value_usd")
            ));
        }
    }
    out
}

fn render_submission_markdown(report: &Value, events: &[StoredEvent]) -> String {
    let mut out = render_run_report_markdown(report);
    out.push_str("\n## Execution Proof\n\n");
    out.push_str(&format!(
        "- Agent wallet: `{}`\n- Policy hash: `{}`\n- Kill switch: `{}`\n",
        text(report, "wallet_address"),
        text(report, "policy_hash"),
        report
            .get("kill_switch")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    ));
    out.push_str("\n## Recent Events\n\n| Time | Event | Payload |\n| --- | --- | --- |\n");
    for event in events.iter().take(30) {
        out.push_str(&format!(
            "| {} | {} | `{}` |\n",
            event.timestamp,
            serde_json::to_value(&event.event_type)
                .ok()
                .and_then(|v| v.as_str().map(ToOwned::to_owned))
                .unwrap_or_else(|| "event".to_string()),
            event.payload_json
        ));
    }
    out
}

fn metric_value(report: &Value, key: &str) -> f64 {
    report
        .get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| report.get(key).and_then(Value::as_f64))
        .unwrap_or(0.0)
}

fn alert_thresholds() -> AlertThresholds {
    let policy = read_json_file(PRODUCTION_POLICY_PATH)
        .or_else(|_| read_json_file(PAPER_POLICY_PATH))
        .unwrap_or_else(|_| json!({}));
    let defaults = AlertThresholds::default();
    let drawdown_soft = metric_value(&policy, "max_total_drawdown_pct") / 100.0;
    let drawdown_hard = metric_value(&policy, "kill_switch_drawdown_pct") / 100.0;
    let slippage_max = metric_value(&policy, "max_slippage_pct") / 100.0;

    AlertThresholds {
        drawdown_soft: nonzero_or(drawdown_soft, defaults.drawdown_soft),
        drawdown_hard: nonzero_or(drawdown_hard, defaults.drawdown_hard),
        data_max_age_secs: 300,
        slippage_max: nonzero_or(slippage_max, defaults.slippage_max),
        recon_max_diff: defaults.recon_max_diff,
    }
}

fn nonzero_or(value: f64, fallback: f64) -> f64 {
    if value > 0.0 {
        value
    } else {
        fallback
    }
}

fn report_age_seconds(report: &Value) -> u64 {
    let updated_ms = report
        .get("updated_ms")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if updated_ms <= 0 {
        return 0;
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    ((now_ms - updated_ms).max(0) / 1000) as u64
}

fn latest_slippage_fraction(events: &[StoredEvent]) -> f64 {
    events
        .iter()
        .find_map(|event| {
            if matches!(event.event_type, AgentEvent::TwakQuoteReceived) {
                Some(metric_value(&event.payload_json, "slippage_pct") / 100.0)
            } else {
                None
            }
        })
        .unwrap_or(0.0)
}

fn alert_inputs(report: &Value, events: &[StoredEvent]) -> AlertInputs {
    let trade_executed = events.iter().any(|event| {
        matches!(event.event_type, AgentEvent::DailyTradeRequirementSatisfied)
            || (matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some())
    });

    AlertInputs {
        drawdown: metric_value(report, "total_drawdown_pct") / 100.0,
        data_age_secs: report_age_seconds(report),
        slippage: latest_slippage_fraction(events),
        recon_diff: 0.0,
        kill_switch: report
            .get("kill_switch")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || events
                .iter()
                .any(|event| matches!(event.event_type, AgentEvent::KillSwitchTriggered)),
        daily_trade_executed: trade_executed,
    }
}

fn active_alerts(report: &Value, events: &[StoredEvent]) -> Vec<Alert> {
    let thresholds = alert_thresholds();
    let inputs = alert_inputs(report, events);
    evaluate_all(&inputs, &thresholds)
}

fn event_count(events: &[StoredEvent], target: AgentEvent) -> usize {
    let target = std::mem::discriminant(&target);
    events
        .iter()
        .filter(|event| std::mem::discriminant(&event.event_type) == target)
        .count()
}

fn readiness_check(id: &str, label: &str, pass: bool, detail: String) -> Value {
    json!({
        "id": id,
        "label": label,
        "status": if pass { "pass" } else { "blocking" },
        "detail": detail
    })
}

fn event_name(event: &AgentEvent) -> String {
    serde_json::to_value(event)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string())
}

fn render_prometheus_metrics(report: &Value, events: &[StoredEvent]) -> String {
    let age_seconds = report_age_seconds(report);
    let tx_count = events
        .iter()
        .filter(|event| {
            matches!(event.event_type, AgentEvent::TxConfirmed)
                && event.payload_json.get("tx_hash").is_some()
        })
        .count();
    let risk_approved = events
        .iter()
        .filter(|event| matches!(event.event_type, AgentEvent::RiskApproved))
        .count();
    let risk_rejected = events
        .iter()
        .filter(|event| matches!(event.event_type, AgentEvent::RiskRejected))
        .count();
    let risk_clipped = events
        .iter()
        .filter(|event| matches!(event.event_type, AgentEvent::RiskClipped))
        .count();
    let positions = report
        .get("positions")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    let mut out = String::new();
    out.push_str("# HELP guardrail_nav_usd Latest reported net asset value in USD.\n");
    out.push_str("# TYPE guardrail_nav_usd gauge\n");
    out.push_str(&format!(
        "guardrail_nav_usd {}\n",
        metric_value(report, "nav_usd")
    ));
    out.push_str(
        "# HELP guardrail_total_drawdown_pct Latest reported total drawdown percentage.\n",
    );
    out.push_str("# TYPE guardrail_total_drawdown_pct gauge\n");
    out.push_str(&format!(
        "guardrail_total_drawdown_pct {}\n",
        metric_value(report, "total_drawdown_pct")
    ));
    out.push_str("# HELP guardrail_report_age_seconds Age of the latest run report in seconds.\n");
    out.push_str("# TYPE guardrail_report_age_seconds gauge\n");
    out.push_str(&format!("guardrail_report_age_seconds {age_seconds}\n"));
    out.push_str("# HELP guardrail_kill_switch Whether the kill switch is active.\n");
    out.push_str("# TYPE guardrail_kill_switch gauge\n");
    out.push_str(&format!(
        "guardrail_kill_switch {}\n",
        if report
            .get("kill_switch")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            1
        } else {
            0
        }
    ));
    out.push_str("# HELP guardrail_events_total Number of recent events visible to the API.\n");
    out.push_str("# TYPE guardrail_events_total gauge\n");
    out.push_str(&format!("guardrail_events_total {}\n", events.len()));
    out.push_str("# HELP guardrail_trades_total Number of confirmed trade transactions visible to the API.\n");
    out.push_str("# TYPE guardrail_trades_total gauge\n");
    out.push_str(&format!("guardrail_trades_total {tx_count}\n"));
    out.push_str(
        "# HELP guardrail_positions Number of open risk positions in the latest run report.\n",
    );
    out.push_str("# TYPE guardrail_positions gauge\n");
    out.push_str(&format!("guardrail_positions {positions}\n"));
    out.push_str(
        "# HELP guardrail_risk_decisions_total Number of recent risk decisions by decision type.\n",
    );
    out.push_str("# TYPE guardrail_risk_decisions_total gauge\n");
    out.push_str(&format!(
        "guardrail_risk_decisions_total{{decision=\"approved\"}} {risk_approved}\n"
    ));
    out.push_str(&format!(
        "guardrail_risk_decisions_total{{decision=\"rejected\"}} {risk_rejected}\n"
    ));
    out.push_str(&format!(
        "guardrail_risk_decisions_total{{decision=\"clipped\"}} {risk_clipped}\n"
    ));
    out.push_str("# HELP guardrail_event_type_total Recent event count by event type.\n");
    out.push_str("# TYPE guardrail_event_type_total gauge\n");
    let mut by_type = std::collections::BTreeMap::<String, usize>::new();
    for event in events {
        *by_type.entry(event_name(&event.event_type)).or_default() += 1;
    }
    for (event_type, count) in by_type {
        out.push_str(&format!(
            "guardrail_event_type_total{{event_type=\"{}\"}} {}\n",
            event_type.replace('"', "\\\""),
            count
        ));
    }
    out
}

fn latest_event<F>(events: &[StoredEvent], predicate: F) -> Option<&StoredEvent>
where
    F: Fn(&StoredEvent) -> bool,
{
    events.iter().find(|event| predicate(event))
}

fn sqlite_path(database_url: &str) -> Option<PathBuf> {
    database_url
        .strip_prefix("sqlite://")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::sqlite_path;
    use std::path::Path;

    #[test]
    fn parses_sqlite_url() {
        assert_eq!(
            sqlite_path("sqlite://data/guardrail_alpha.db").as_deref(),
            Some(Path::new("data/guardrail_alpha.db"))
        );
    }

    #[test]
    fn rejects_non_sqlite_url() {
        assert!(sqlite_path("postgres://localhost/db").is_none());
    }
}
