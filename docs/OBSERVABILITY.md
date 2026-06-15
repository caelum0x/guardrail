# Observability

Guardrail Alpha exposes runtime health through a Prometheus exporter, a set of
alert rules, Grafana dashboards, and an in-process watchdog. Everything is
read-only: the observability surface never trades and never writes agent state.

## Components

| Component | Path | Role |
| --- | --- | --- |
| `guardrail-exporter` | `apps/guardrail-exporter` | HTTP sidecar exposing `/metrics` (Prometheus text format) and `/healthz`. |
| `guardrail-monitor` | `apps/guardrail-monitor` | Watchdog that loads the run report and logs alerts (staleness, drawdown, kill switch). |
| Prometheus config | `infra/prometheus/` | Scrape jobs (`prometheus.yml`) and alert rules (`alerts.yml`). |
| Grafana | `infra/grafana/` | Provisioned datasource and three dashboards. |

## Exporter

The exporter derives metrics from two sources the agent already produces:

1. The SQLite **event log** (default `sqlite://data/guardrail_alpha.db`) — counted
   over the most recent `10,000` events.
2. The **run report** JSON (default `data/run_report.json`) — NAV, drawdown,
   positions, kill switch, and freshness.

### Configuration (environment)

| Variable | Default | Meaning |
| --- | --- | --- |
| `DATABASE_URL` | `sqlite://data/guardrail_alpha.db` | Event-log SQLite database. |
| `GUARDRAIL_REPORT` | `data/run_report.json` | Run report path. |
| `EXPORTER_ADDR` | `0.0.0.0:9100` | Bind address for the HTTP server. |
| `RUST_LOG` | `info` | Tracing filter. |

On any read error (missing DB or report) the exporter degrades gracefully:
event counts return zero and report-derived gauges are simply omitted.

### Exposed metrics

All metrics are emitted as Prometheus **gauges**. The count metrics are absolute
snapshots over the scanned window (they carry a `_total` suffix by convention but
are reported as gauges, not counters).

**From the event log:**

| Metric | Meaning |
| --- | --- |
| `guardrail_events_total` | Total recorded agent events (window size). |
| `guardrail_trades_total` | Confirmed on-chain swaps (`TxConfirmed`). |
| `guardrail_risk_rejections_total` | Orders rejected by the risk engine (`RiskRejected`). |
| `guardrail_orders_proposed_total` | Orders proposed by the strategy (`OrderProposed`). |
| `guardrail_quotes_total` | TWAK quotes received (`TwakQuoteReceived`). |
| `guardrail_daily_trade_satisfied_total` | Cycles satisfying the daily-trade requirement (`DailyTradeRequirementSatisfied`). |

**From the run report:**

| Metric | Meaning |
| --- | --- |
| `guardrail_nav_usd` | Net asset value in USD. |
| `guardrail_total_drawdown_pct` | Total drawdown percent. |
| `guardrail_positions` | Number of open non-reserve positions. |
| `guardrail_position_weight_pct{symbol="..."}` | Per-asset position weight as percent of NAV (one series per symbol). |
| `guardrail_kill_switch` | Kill switch engaged (`1`) or armed (`0`). |
| `guardrail_report_age_seconds` | Seconds since the last run report update. |

Report-derived gauges appear only when the report is present and the field is
parseable (string-or-number fields are coerced to float).

### Running the exporter

```bash
# Build and run locally (binds 0.0.0.0:9100 by default)
cargo run -p guardrail-exporter

# Override paths/address
DATABASE_URL=sqlite://data/guardrail_alpha.db \
GUARDRAIL_REPORT=data/run_report.json \
EXPORTER_ADDR=0.0.0.0:9100 \
  cargo run -p guardrail-exporter

# Or via docker-compose (service "exporter", published on host :9100)
docker compose up -d exporter
```

### Scraping `/metrics`

```bash
# Liveness
curl -s http://localhost:9100/healthz        # -> ok

# Full Prometheus exposition body
curl -s http://localhost:9100/metrics

# Spot-check a single metric
curl -s http://localhost:9100/metrics | grep guardrail_nav_usd
```

## Prometheus

`infra/prometheus/prometheus.yml` sets a `15s` scrape and evaluation interval and
loads `alerts.yml` as rules.

### Scrape jobs

| Job | Target | Path |
| --- | --- | --- |
| `guardrail-api` | `api:8080` | `/metrics` |
| `guardrail-exporter` | `exporter:9100` | `/metrics` |
| `prometheus` | `localhost:9090` | (self) |

Targets use the docker-compose service DNS names. Prometheus is published on host
port `9090`.

### Alert rules (`alerts.yml`, group `guardrail-alpha`)

| Alert | Expression | For | Severity |
| --- | --- | --- | --- |
| `MetricsSurfaceDown` | `up{job=~"guardrail-api\|guardrail-exporter"} == 0` | 1m | critical |
| `ReportStale` | `guardrail_report_age_seconds > 300` | 2m | warning |
| `DrawdownSoftBreach` | `guardrail_total_drawdown_pct > 10` | 2m | warning |
| `DrawdownHardBreach` | `guardrail_total_drawdown_pct > 20` | 1m | critical |
| `KillSwitchEngaged` | `guardrail_kill_switch == 1` | 30s | critical |
| `NoRecentTrades` | `increase(guardrail_trades_total[1h]) == 0` | 1h | warning |

## Grafana

Provisioning under `infra/grafana/provisioning/`:

- **Datasource** (`datasources/prometheus.yml`): a default Prometheus datasource
  (uid `prometheus`) at `http://prometheus:9090`.
- **Dashboards** (`dashboards/dashboards.yml`): file provider loading JSON from
  `/var/lib/grafana/dashboards` into the "Guardrail Alpha" folder, refreshed
  every 30s.

Grafana runs on container port `3000`, published on host port **`3001`** in
docker-compose.

### Dashboards

| File | Title | Panels |
| --- | --- | --- |
| `agent-health.json` | Guardrail Alpha — Agent Health | Metrics Surfaces Up (`up{...}`), Events Recorded (`guardrail_events_total`), Snapshot Age (`guardrail_report_age_seconds`), Events Recorded over time. |
| `pnl.json` | Guardrail Alpha — PnL & NAV | Net Asset Value (`guardrail_nav_usd`), Open Positions (`guardrail_positions`), Confirmed Trades (`guardrail_trades_total`), Allocation by Asset (`guardrail_position_weight_pct`). |
| `trading-risk.json` | Guardrail Alpha — Trading Risk | Total Drawdown (`guardrail_total_drawdown_pct`), Kill Switch (`guardrail_kill_switch`), Risk Decisions / rejections (`guardrail_risk_rejections_total`). |

## Monitor watchdog

`guardrail-monitor` is a standalone watchdog (separate from the exporter). Each
cycle it loads the run report and evaluates three **pure, side-effect-free**
checks (`checks.rs`), then logs any raised alerts via `tracing` (`watchdog.rs`).

| Check | Condition | Severity | Notes |
| --- | --- | --- | --- |
| `report_is_stale` | report age `> 60s` (`MAX_AGE_MS`) | Warning | Disabled if `max_age_ms <= 0`. |
| `drawdown_breach` | `\|drawdown\| >= 20%` (hard) else `>= 10%` (soft) | Critical / Warning | Hard takes precedence; sign-agnostic; `None` if field absent/unparseable. |
| `kill_switch_active` | `kill_switch == true` | Critical | — |

A missing report logs a warning and the loop continues; read/parse errors are
logged but swallowed so the watchdog stays resilient. A clear cycle logs
`watchdog clear: no alerts` with run id, mode, and regime.

### Configuration (environment)

| Variable | Default | Meaning |
| --- | --- | --- |
| `GUARDRAIL_REPORT` | `data/run_report.json` | Run report path. |
| `GUARDRAIL_MONITOR_CHECKS` | `3` | Number of watchdog cycles before exit; `0` runs forever. |

Cycles are spaced `5s` apart (`CYCLE_INTERVAL`).

```bash
# Run 3 cycles (default) then exit
cargo run -p guardrail-monitor

# Run continuously
GUARDRAIL_MONITOR_CHECKS=0 cargo run -p guardrail-monitor
```
