# Guardrail Alpha — full local stack (Docker Compose)

A single Compose file that runs the **entire** Guardrail stack on one machine:
the trading agent, its read-only sidecars (API, metrics exporter, monitor), the
dashboard UI, an optional Prometheus + Grafana monitoring stack, and an optional
alert relay.

It mirrors the production topology in [`deploy/k8s`](../k8s) and
[`deploy/helm`](../helm), but wires everything together for local development
and demos. For the minimal core-only compose, see the repository root
[`docker-compose.yml`](../../docker-compose.yml); this file is the **complete**
superset (adds named data volume, healthchecks, monitoring + alerts profiles,
and `.env`-driven configuration).

## TL;DR

```bash
# 1. Create your env file from the template (non-secret defaults).
cp deploy/compose/.env.example deploy/compose/.env

# 2. Build + start the core stack (agent, api, exporter, monitor, dashboard).
docker compose -f deploy/compose/docker-compose.full.yml up --build

# 3. Open the UI and API.
#    Dashboard  -> http://localhost:3000
#    API        -> http://localhost:8080/health
#    Exporter   -> http://localhost:9100/metrics
```

Stop and remove everything (keep data):

```bash
docker compose -f deploy/compose/docker-compose.full.yml down
```

Stop and wipe the shared data volume too:

```bash
docker compose -f deploy/compose/docker-compose.full.yml down -v
```

## Services & ports

| Service | Profile | Host port | Purpose |
|---|---|---|---|
| `agent` | (default) | _none_ | Trading agent — **sole writer** of the shared data volume. No published port by design. |
| `api` | (default) | `8080` | Read-only JSON/HTTP API. Has a `/health` healthcheck. |
| `exporter` | (default) | `9100` | Prometheus metrics (`/metrics`). |
| `monitor` | (default) | _none_ | Watchdog / readiness sidecar. Reads the run report. |
| `dashboard` | (default) | `3000` | Next.js read-only UI (browser talks to the API). |
| `prometheus` | `monitoring` | `9090` | Scrapes `api:8080` + `exporter:9100` using `infra/prometheus`. |
| `grafana` | `monitoring` | `3001` | Provisioned dashboards from `infra/grafana`. |
| `alert-relay` | `alerts` | _none_ | Polls the API `/alerts` feed and forwards to chat/email sinks. |

Ports are configurable via `.env` (`API_PORT`, `DASHBOARD_PORT`, …) if any
collide with something already running on your host.

## Profiles

The core five services start unconditionally. The rest are **opt-in** via Compose
profiles, so you only run what you need.

### Monitoring (Prometheus + Grafana)

```bash
docker compose -f deploy/compose/docker-compose.full.yml --profile monitoring up
```

- Prometheus: <http://localhost:9090>
- Grafana: <http://localhost:3001> (user `admin`, password from
  `GRAFANA_ADMIN_PASSWORD` in `.env` — defaults to `guardrail` for local use).

Prometheus reuses the scrape config and alert rules under
[`infra/prometheus`](../../infra/prometheus); Grafana reuses the datasource +
dashboards under [`infra/grafana`](../../infra/grafana).

### Alerts (alert relay)

```bash
docker compose -f deploy/compose/docker-compose.full.yml --profile alerts up
```

The relay is **offline-safe by default**: it runs the dry-run loop and makes no
calls to any sink. To deliver for real, set `RELAY_ALERT_ARGS=--live` in `.env`,
enable the desired sinks in your alert config, and supply the matching sink
secrets (`GUARDRAIL_SLACK_WEBHOOK_URL`, `GUARDRAIL_SMTP_*`, …). The image
contains **no secrets** — credentials are injected at runtime and referenced by
the config via `env:VAR_NAME`. See
[`integrations/alert-relay/README.md`](../../integrations/alert-relay/README.md).

### Everything at once

```bash
docker compose -f deploy/compose/docker-compose.full.yml \
  --profile monitoring --profile alerts up --build
```

## Configuration

All configuration is read from `deploy/compose/.env` (copied from
[`.env.example`](./.env.example)). That template ships only **non-secret
defaults and empty placeholders**. Fill in real values (CMC key, BSC RPC URL,
alert sink credentials, …) in your local `.env`, which is git-ignored.

Key knobs:

- `AGENT_CONFIG` — which file under `configs/` the agent loads (default
  `configs/paper.toml`). The repo `configs/` directory is mounted read-only.
- `DATABASE_URL` / `GUARDRAIL_REPORT` — the shared event log + run report paths.
- `NEXT_PUBLIC_API_URL` — browser-facing API URL for the dashboard.
- `*_PORT` — published host ports (override on collisions).
- `GRAFANA_ADMIN_PASSWORD` — change before exposing Grafana off localhost.

## Data & the single-writer rule

`agent`, `api`, `exporter`, and `monitor` share one named volume
(`guardrail-data`) holding the SQLite event log and run report. The **agent is
the only writer**; the others read. Do not run a second agent against the same
volume.

## Verify the stack

```bash
# Validate the compose file without starting anything.
docker compose -f deploy/compose/docker-compose.full.yml config -q

# Watch health once up.
docker compose -f deploy/compose/docker-compose.full.yml ps
curl -fsS http://localhost:8080/health
curl -fsS http://localhost:9100/metrics | head
```

## Where to go next

For the full deployment matrix (compose vs Kubernetes vs Helm vs single-host
systemd) and guidance on when to use each, see
[`docs/DEPLOYMENT.md`](../../docs/DEPLOYMENT.md).
