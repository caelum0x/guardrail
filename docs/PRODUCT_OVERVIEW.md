# Product Overview

Guardrail Alpha is an autonomous trading agent for BNB Smart Chain (chain id
`56`) built around one non-negotiable invariant: **the Rust risk engine is the
sole execution gate, and the Trust Wallet Agent Kit (TWAK) is the sole
executor.** Strategy code produces *intent*; only the risk engine can mint the
`ApprovedOrder` that TWAK requires to sign a swap, so executing without a risk
approval is a compile error rather than a runtime hope.

Around that live core sits a full product: an analytics lab, two dashboards, a
catalog of advisory strategy skills with a regime ensemble, an ecosystem of
read-only SDKs (TypeScript / Python / Go), an MCP server for the CoinMarketCap
Agent Hub, BNB identity + an independent proof verifier, a TWAK self-custody
demo, and an ops layer (alert relay, stress scenarios, Helm/Kustomize,
Prometheus/Grafana). Everything in this document is **offline-safe** — it runs
in paper mode with deterministic mocks and needs no API keys or chain access.

This file is the high-level tour. Deeper references:
[ARCHITECTURE.md](ARCHITECTURE.md) (crate graph + trust boundaries),
[PRIZE_MAP.md](PRIZE_MAP.md) (evidence table), [OPERATIONS.md](OPERATIONS.md)
(operator runbook), [WHATS_NEW.md](WHATS_NEW.md) (changelog), and
[FEATURE_MATRIX.md](FEATURE_MATRIX.md) (capability matrix).

---

## At a glance

| Layer | What it is | Where it lives |
|-------|-----------|----------------|
| Live engine | Rust Cargo workspace: 19 crates + 9 binaries | `crates/`, `apps/` |
| Risk gate | Sole authority between intent and execution | `crates/risk-engine` |
| Executor | TWAK self-custody signer (Mock/REST/MCP/CLI transports) | `crates/twak-client` |
| Analytics | Python lab over the event log + run report | `python-lab/` |
| Dashboards | Next.js read-only cockpit (55 pages) + zero-build web-lite | `dashboard/`, `clients/web-lite/` |
| Track-2 skills | 4 advisory strategy skills + regime ensemble + authoring kit | `skills/` |
| Ecosystem clients | TS / Python / Go SDKs, MCP, LangChain, Postman | `clients/` |
| Identity / proof | BNB agent identity + independent proof verifier | `crates/bnb-agent`, `clients/proof-verifier` |
| Self-custody | TWAK signing policy + narrated demo | `configs/signing_policy.example.json`, `scripts/self_custody_demo.sh` |
| Ops / observability | Alert relay, scenarios, exporter, Helm/k8s | `integrations/`, `configs/scenarios/`, `infra/`, `deploy/` |

---

## System map

```
                         +---------------------------------------------+
                         |              LIVE ENGINE (Rust)             |
                         |                                             |
   CMC data  ──────────► |  cmc-client ─► market-data ─► feature-      |
   (REST / MCP / x402 /  |                               engine ─►     |
    Mock)                |                               strategy-     |
                         |                               engine        |
                         |                                  │ intent   |
                         |                                  ▼          |
                         |   portfolio ─► RISK-ENGINE  ◄── THE ONLY    |
                         |                (pre_trade +      GATE        |
                         |                 final_quote_check)          |
                         |                       │ ApprovedOrder       |
                         |                       ▼                     |
                         |                 twak-client ── THE ONLY     |
                         |                 (signs w/ user keys) EXEC    |
                         |                       │                     |
                         |                       ▼ fill                |
                         |                 portfolio reconcile         |
                         +-------------------┬-------------------------+
                                             │ emits (never reads back)
                       ┌─────────────────────┴───────────────────────┐
                       ▼                                              ▼
            event-store (SQLite:                          data/run_report.json
            data/guardrail_alpha.db)                      (NAV, drawdown, kill switch)
                       │                                              │
       ┌───────────────┼──────────────┬─────────────┬───────────────┤
       ▼               ▼              ▼             ▼               ▼
  guardrail-api   guardrail-     guardrail-    guardrail-      python-lab
  (52 routes,     exporter       monitor       tui / replay    (analytics:
   read-only)     /metrics:9100  ─► notifier   (terminal)       regime, drawdown,
       │              │              │                          montecarlo, ensemble,
       │              ▼              ▼                          journal, dossier)
       │          Prometheus    alert-relay
       │          ─► Grafana    (─► chat sinks)
       │
   ┌───┴──────────────┬───────────────┬──────────────┬─────────────┐
   ▼                  ▼               ▼              ▼             ▼
 dashboard       web-lite        clients/ts /     clients/mcp    clients/proof-
 (Next.js,       cockpit         python / go      (CMC Agent     verifier
  55 pages)      (single file)   SDKs (read-only) Hub server)    (clean-room)
```

Authority flows in exactly one direction. Strategy never depends on the
executor, the executor only accepts engine-minted approvals, and every consumer
downstream of the engine is read-only — none has a path back into the trading
loop.

---

## The pieces

### 1. The Rust live engine (`crates/`, `apps/`)

A 19-crate Cargo workspace with a one-way dependency graph (full graph in
[ARCHITECTURE.md](ARCHITECTURE.md)). The canonical loop is
`agent-runtime::AgentRuntime::run_cycle`: pull market data → mark portfolio +
update drawdown → strategy decides intent → **risk gate per order** (twice:
pre-trade and post-quote) → TWAK signs/executes → reconcile fills → append every
step to the event store. Nine binaries sit on top (`apps/`): `guardrail-agent`
(the only one that trades), plus `guardrail-api`, `guardrail-cli`,
`guardrail-tui`, `guardrail-monitor`, `guardrail-exporter`, `guardrail-replay`,
`guardrail-sim`, and `guardrail-doctor`.

- **Risk engine as the sole gate.** `crates/risk-engine` runs a policy + a check
  suite (allowlist, drawdown, stable reserve, position cap, wallet balance,
  kill switch). `RiskEngine::approve` runs `pre_trade` then `final_quote_check`;
  no non-rejected decision means no swap.
- **TWAK as the sole executor.** `crates/twak-client::TwakExecutor::execute_swap`
  takes a `&ApprovedOrder` — a type only the risk engine can produce. Transports:
  Mock (offline default), REST, MCP, CLI. x402 payment authorizations for premium
  CMC data are also TWAK-signed.

### 2. The analytics layer (`python-lab/`)

Analytics-only, with no trading path. `python-lab/analyze.py` is a CLI over the
same source of truth the agent writes (`data/guardrail_alpha.db`,
`data/run_report.json`) with seven subcommands: `regime`, `drawdown`,
`montecarlo`, `dossier`, `ensemble`, `ensemble-compare`, `journal`. The reusable
library lives in `python-lab/guardrail_lab/` (including the `ensemble.py`
blender and the `skill` validator).

### 3. The dashboards (`dashboard/`, `clients/web-lite/`)

- **Next.js dashboard** (`dashboard/`) — read-only, 55 page routes under
  `dashboard/src/app/` (cockpit, portfolio, signals, backtest, walkforward,
  scenarios, proof, prizes, etc.). It only renders `guardrail-api` responses.
- **web-lite cockpit** (`clients/web-lite/index.html`) — a single-file,
  zero-build cockpit wired to the same API, including Ensemble, Journal, and
  Signing tabs.

### 4. Track-2 skills + ensemble (`skills/`)

Four advisory-only strategy skills enumerated in `skills/INDEX.json`:
`regime-routed-bsc-alpha`, `funding-rate-carry-bsc`, `mean-reversion-chop-bsc`,
and `trend-breakout-momentum-bsc`. A regime ensemble meta-allocator
(`skills/ensemble.json` + `python-lab/guardrail_lab/ensemble.py`) blends their
example books by classified regime. A skill authoring kit (`skills/_template/`,
`scripts/new_skill.sh`, `scripts/lint_skills.sh`) lets a judge scaffold and lint
a fifth skill. Every skill is advisory — the risk engine stays the only gate.

### 5. Ecosystem clients (`clients/`)

Nine entries under `clients/`: `typescript` (`@guardrail/client`), `python`
(`guardrail_client`), `go` (read-only Go SDK), `mcp` (CMC Agent Hub server),
`langchain` (tool wrappers), `postman` (collection), `proof-verifier`
(clean-room verifier), `web-lite` (cockpit), and shared `examples`. The
TS/Python/Go SDKs and dashboards are all read-only consumers of `guardrail-api`.

### 6. Identity / self-custody (BNB proof + TWAK)

- **BNB identity** — `crates/bnb-agent` builds `AgentIdentity`, `AgentMetadata`,
  ERC-8004 / ERC-8183 records, and an `AgentProof` with `policy_hash`,
  `report_hash`, and BscScan links.
- **Independent proof verifier** — `clients/proof-verifier/verify.py` is a
  stdlib-only, clean-room tool that re-derives the hashes and URL formats from
  first principles ("don't trust, verify"), run via `scripts/verify_proof.sh`.
- **TWAK self-custody** — `configs/signing_policy.example.json` documents the
  signing envelope (caps, allowed/forbidden actions, x402 allow/deny-list);
  `scripts/self_custody_demo.sh` narrates the agent-proposes → risk-gates →
  TWAK-signs → reconcile flow without ever loading a key.

### 7. Ops (alert relay, scenarios, helm/k8s)

- **Alert relay** — `integrations/alert-relay/relay.py` is an out-of-process,
  read-only consumer of `GET /alerts` that dedups, filters by severity, and
  forwards to chat sinks (dry-run by default).
- **Stress scenarios** — `configs/scenarios/` (`index.json` + per-scenario JSON)
  driven by `scripts/run_scenarios.sh` / `guardrail-sim`, served at
  `GET /scenarios`.
- **Observability** — `guardrail-exporter` on `:9100`, with
  `infra/prometheus/` + `infra/grafana/` configs.
- **Deploy** — `deploy/helm/guardrail` (Helm chart) and `deploy/k8s`
  (Kustomize), plus `docker-compose.yml` and per-binary Dockerfiles in `infra/`.

---

## Where things live (directory guide)

| Path | Contents |
|------|----------|
| `crates/` | 19 Rust crates — the live engine (risk, strategy, twak, bnb-agent, …) |
| `apps/` | 9 binaries — agent, api, cli, tui, monitor, exporter, replay, sim, doctor |
| `agent-runtime` (crate) | Top crate composing every lower crate into the live loop |
| `dashboard/` | Next.js read-only dashboard (55 pages under `src/app/`) |
| `python-lab/` | Analytics CLI (`analyze.py`) + `guardrail_lab/` library |
| `skills/` | 4 strategy skills, `INDEX.json`, `ensemble.json`, `_template/` |
| `clients/` | TS / Python / Go SDKs, MCP server, LangChain, Postman, web-lite, proof-verifier |
| `integrations/` | `alert-relay/` (watchdog relay) + `bnbagent-sdk/` |
| `configs/` | Risk policies, eligible assets, signing policy, `scenarios/` |
| `scripts/` | Operator + demo scripts (`guardrail.sh`, `judge_quickstart.sh`, …) |
| `infra/` | Dockerfiles, Prometheus, Grafana, systemd units |
| `deploy/` | `helm/guardrail` chart + `k8s` Kustomize manifests |
| `docs/` | Architecture, prize map, runbooks, ADRs (`docs/adr/`) |
| `data/` | Runtime artifacts: `guardrail_alpha.db`, `run_report.json` |

---

## One-command tour

```bash
# Build + run paper agent + serve API and web-lite cockpit (offline).
scripts/judge_quickstart.sh

# Unified operator front door (subcommands: up, cockpit, analyze, scenarios,
# verify, alerts, skills, new-skill).
scripts/guardrail.sh up

# Full offline E2E demo (all evidence in one run).
scripts/demo.sh
```
