# Guardrail Grafana

Provisioned Grafana assets for the Guardrail Alpha stack. Everything is
zero-touch: bringing up the `monitoring` Compose profile gives a Grafana that
already has the Prometheus data source wired and all dashboards loaded, no
manual clicking required.

## Provisioning layout

```
infra/grafana/
├── dashboards/                       # dashboard JSON (the actual panels)
│   ├── pnl.json
│   ├── trading-risk.json
│   ├── agent-health.json
│   └── alerts.json
├── provisioning/
│   ├── datasources/
│   │   └── datasource.yml            # Prometheus datasource (default)
│   └── dashboards/
│       └── dashboards.yml            # file provider that loads ../dashboards
└── README.md
```

How it is mounted by `deploy/compose/docker-compose.full.yml` (grafana service,
`monitoring` profile), all read-only:

| Host path | Container path | Purpose |
|-----------|----------------|---------|
| `infra/grafana/provisioning` | `/etc/grafana/provisioning` | datasource + dashboard provider configs (Grafana auto-loads every `*.yml` here on startup) |
| `infra/grafana/dashboards` | `/var/lib/grafana/dashboards` | the dashboard JSON the file provider reads |

- **Data source** (`provisioning/datasources/datasource.yml`): a single
  Prometheus data source with `uid: prometheus`, set as default and pointing at
  the Compose Prometheus service `http://prometheus:9090`. The fixed `uid` is
  what every dashboard panel references, so the dashboards resolve their queries
  without any per-environment edits.
- **Dashboard provider** (`provisioning/dashboards/dashboards.yml`): a `file`
  provider that scans `/var/lib/grafana/dashboards` (the mounted `dashboards/`
  dir) every 30s and loads each JSON into the `Guardrail Alpha` folder.

To add a dashboard, drop a new JSON file into `dashboards/` — the provider picks
it up automatically; no datasource or compose change is needed.

## Dashboards

| File | UID | Focus |
|------|-----|-------|
| `dashboards/pnl.json` | `guardrail-pnl` | NAV, positions, trades, allocation |
| `dashboards/trading-risk.json` | `guardrail-risk` | Drawdown, kill switch, risk rejections |
| `dashboards/agent-health.json` | `guardrail-health` | Surface up/down, events, snapshot age |
| `dashboards/alerts.json` | `guardrail-alerts` | Cross-cutting overview: NAV, drawdown, regime, alert counts, trade count |

## Metric names

The exporter (`apps/guardrail-exporter`) currently exposes these gauges/counters,
all used directly by the dashboards:

- `guardrail_nav_usd`
- `guardrail_total_drawdown_pct`
- `guardrail_trades_total`
- `guardrail_kill_switch`
- `guardrail_positions`
- `guardrail_position_weight_pct`
- `guardrail_risk_rejections_total`
- `guardrail_orders_proposed_total`
- `guardrail_quotes_total`
- `guardrail_daily_trade_satisfied_total`
- `guardrail_events_total`
- `guardrail_report_age_seconds`

## Placeholder metrics (alerts dashboard)

`dashboards/alerts.json` references two metrics that the exporter does **not**
emit yet. They are intentionally clearly named so that when the exporter starts
publishing them the panels light up with no dashboard edits. Until then those
two panels render "No data".

| Placeholder metric | Panel | What it represents | Where the data lives today |
|--------------------|-------|--------------------|----------------------------|
| `guardrail_market_regime` | "Market Regime" | Numeric regime code (e.g. `0=calm`, `1=volatile`, `2=stress`) | Computed by the agent; not yet exported as a gauge |
| `guardrail_active_alerts{severity=...}` | "Active Alert Counts by Severity" | Count of currently-active alerts per severity | Surfaced by the API `/alerts` `counts` field and consumed by `integrations/alert-relay`; not yet exported to Prometheus |

To wire these up, add the two gauges to `apps/guardrail-exporter` (mirroring the
existing `add_gauge`/`# HELP`/`# TYPE` pattern) using exactly these names and, for
`guardrail_active_alerts`, a `severity` label of `info` / `warning` / `critical`.

## Template variables

`dashboards/alerts.json` defines two template variables:

- `$datasource` — selects the Prometheus data source (defaults to `prometheus`).
- `$severity` — filters the alert-count panel by `info` / `warning` / `critical`.
