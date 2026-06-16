# Architecture

Guardrail Alpha is a Rust-first autonomous trading agent for BNB Smart Chain
(chain id `56`). It is a Cargo workspace of focused crates, **nine binaries**
(`apps/`), a read-only Next.js dashboard, two read-only client SDKs
(TypeScript + Python), an OpenAPI spec, and a Python analytics lab.

## Crate graph (one-way dependency)

Dependencies flow in a single direction — data and types move "up" the stack,
authority never flows back down. No crate below the risk engine ever calls into
strategy, and the strategy crate never depends on the executor.

```
common                         # shared types: OrderIntent, OrderSide,
  │                            #   TargetPosition, QuoteSummary, Asset, money,
  │                            #   ids, time, Settings, constants
  ├── indicators               # classic technical indicators over a price
  │                            #   series (pure math; depends only on common)
  ├── cmc-client               # CmcDataSource trait + REST/MCP client + mock + x402
  │     └── market-data        # normalizes CMC into MarketSnapshot / RegimeInputs
  │           ├── feature-engine     # snapshot -> per-asset 0..1 feature scores
  │           │     └── strategy-engine  # regime + alpha + allocator + rebalance
  │           │           (also uses portfolio-optimizer for target weights)
  │           ├── portfolio          # NAV, holdings, drawdown, trade accounting
  │           │     └── risk-engine  # THE GATE: policy + checks + approval
  │           │           ├── twak-client   # TwakExecutor trait + mock (depends
  │           │           │                 #   on risk for ApprovedOrder type)
  │           │           └── execution     # orchestrates intent->risk->quote->
  │           │                             #   final risk->execute->reconcile
  │           └── backtester         # reuses strategy + risk + portfolio over a
  │                                  #   synthetic path; buy-and-hold benchmark
  ├── portfolio-optimizer      # standalone weight optimizer (serde-only); used
  │                            #   by strategy-engine + agent-runtime + the API
  ├── event-store              # SQLite + in-memory append-only AgentEvent log
  ├── policy-compiler          # NL mandate / JSON -> validated RiskPolicy + hash
  ├── bnb-agent                # identity, metadata, ERC-8004/8183 records, hashes
  ├── llm-interface            # advisory-only LLM boundary (drafting/explain)
  ├── notifier                 # standalone outbound alert sink (reqwest); used
  │                            #   by guardrail-monitor to dispatch alerts
  └── observability            # tracing, metrics, health, alerts

agent-runtime                  # top crate: wires every crate into the live loop
apps/*                         # nine binaries on top of the crates (see below)
```

`agent-runtime` sits at the top of the crate graph (not an `app`); it composes
every lower crate into the live trading loop. The binaries in `apps/` depend on
`agent-runtime` and the lower crates but nothing depends on them.

The three newer crates are deliberately decoupled:

- **`indicators`** — pure technical-indicator math, depends only on `common`;
  consumed by `guardrail-api` (`/indicators`) and `guardrail-cli` (`indicators`).
- **`portfolio-optimizer`** — standalone (serde-only) target-weight optimizer;
  consumed by `strategy-engine`, `agent-runtime`, and `guardrail-api`
  (`/optimize`). It has no market or execution dependencies.
- **`notifier`** — standalone outbound alert sink (reqwest/async); consumed by
  `guardrail-monitor` to deliver watchdog alerts. It has no path into the
  trading loop.

## Binary inventory (`apps/`)

| Binary | Role | Trades? |
|--------|------|---------|
| `guardrail-agent`    | Runs `agent-runtime` — the live trading loop | Yes (only one) |
| `guardrail-api`      | axum read API over the event store + run report (69 routes) | No |
| `guardrail-cli`      | Dev/admin CLI: backtest, compare, score, quote, walk-forward, markets, indicators, experiments, register, identity, policy hash/compile, kill-switch, report, submission | No |
| `guardrail-tui`      | Terminal cockpit: polls the run report + event totals and renders to the terminal; refreshes a fixed number of times then exits cleanly | No |
| `guardrail-monitor`  | Watchdog: polls run report, raises staleness / drawdown / kill-switch alerts, dispatches via `notifier` | No |
| `guardrail-exporter` | Prometheus exporter sidecar: derives gauges from SQLite log + run report on `/metrics` (`:9100`) | No |
| `guardrail-replay`   | Event-log audit: chronological journal, trade table, CSV export, summary | No |
| `guardrail-sim`      | Scenario sweep across Fear & Greed inputs; `--walk-forward` window sequence | No |
| `guardrail-doctor`   | Preflight checks: config load, risk-policy validation, universe, data-dir writability | No |

## Data / trade / risk / event flow (one cycle)

`agent-runtime::AgentRuntime::run_cycle` is the canonical pipeline:

1. **Market data** — `SnapshotBuilder` pulls from a `CmcDataSource` and
   `market-data` normalizes it into a `MarketSnapshot`; `validator::validate`
   rejects stale/empty snapshots (the cycle is skipped, never traded blind).
2. **Mark + drawdown** — the portfolio is marked to current prices and
   `DrawdownTracker` updates.
3. **Risk monitor** — if total drawdown crosses `kill_switch_drawdown_pct` the
   kill switch latches and a `KillSwitchTriggered` event is emitted; if it
   crosses `max_total_drawdown_pct` the cycle is **throttled** (a
   `DrawdownThrottleActivated` event is emitted and Buy intents are suppressed).
4. **Strategy** — `StrategyEngine::decide` produces a `StrategyDecision`
   (regime, target weights via `portfolio-optimizer`, `Vec<OrderIntent>`,
   explanation). Intent only — no authority.
5. **Per-order gate** — each `OrderIntent` runs the risk pipeline (below).
6. **Daily-trade heartbeat** — if nothing executed this cycle and the policy
   requires daily activity, a small compliant heartbeat order is injected and
   sent through the same gate.
7. **Reconcile + log** — fills update the portfolio; every step is appended to
   the event store as an `AgentEvent`.

## The risk engine is the only gate

`risk-engine` is the sole authority boundary between strategy intent and
execution. An order reaches TWAK only after `RiskEngine` returns a non-rejected
decision twice: once pre-trade and once after the quote — `RiskEngine::approve`
runs `pre_trade` then `final_quote_check`. No `RiskDecision::Approved` (or
`Clipped`) means no swap. This rule is reinforced structurally: `strategy-engine`
does not depend on `twak-client` or `execution`, so it has no path to call the
executor directly. See [RISK.md](./RISK.md) for the full check list and policy.

## TWAK is the sole executor

The Trust Wallet Agent Kit (`twak-client`) is the only execution layer; nothing
else can submit a swap. Self-custody is enforced by the type system:
`TwakExecutor::execute_swap` requires an engine-minted `ApprovedOrder`, a type
only the risk engine can produce, so executing without a risk approval is a
**compile error**. TWAK holds the keys and signs; the engine only builds intents
and approvals. Transports are Mock (offline default), REST, MCP, and CLI; x402
payment authorizations for premium CMC data are also TWAK-signed. See
[TWAK_INTEGRATION.md](./TWAK_INTEGRATION.md), [SELF_CUSTODY.md](./SELF_CUSTODY.md),
and [ADR 0003](adr/0003-twak-only-execution.md).

## Observability path

The agent emits two artifacts that everything downstream reads; it never reaches
back into the trading path.

```
agent-runtime
  ├── event-store (SQLite: data/guardrail_alpha.db)   # append-only AgentEvent log
  └── data/run_report.json                            # NAV, drawdown, positions, kill switch

guardrail-exporter  ── reads both ──► /metrics  ──►  Prometheus  ──►  Grafana
guardrail-monitor   ── reads run report ──► staleness / drawdown / kill-switch alerts ──► notifier
guardrail-api       ── reads both ──► JSON + markdown endpoints
guardrail-tui       ── reads both ──► terminal cockpit
dashboard (Next.js) ── reads guardrail-api ──► read-only pages
clients/{ts,python} ── read guardrail-api ──► typed read-only SDKs
```

Prometheus scrape config and Grafana datasource/dashboards live under
`infra/prometheus/` and `infra/grafana/`. The exporter listens on `:9100` and
exposes gauges such as `guardrail_events_total`, `guardrail_trades_total`,
`guardrail_risk_rejections_total`, `guardrail_nav_usd`,
`guardrail_total_drawdown_pct`, `guardrail_position_weight_pct{symbol=...}`,
`guardrail_kill_switch`, and `guardrail_report_age_seconds`. See
[OBSERVABILITY.md](./OBSERVABILITY.md) for the full metric catalog.

## API endpoint inventory (`apps/guardrail-api`)

The 69 read-only `GET` routes over the event store and run report are listed in
full in [API.md](API.md); a representative subset is shown below. Source of
truth: `apps/guardrail-api/src/server.rs::build_app`.

| Endpoint | Purpose |
|----------|---------|
| `/health` | Liveness + DB reachability |
| `/portfolio` | Current NAV, holdings, weights |
| `/trades` | Confirmed on-chain swaps |
| `/signals` | Latest regime + per-asset alpha scores |
| `/risk` | Active policy snapshot + recent rejections |
| `/alerts` | Watchdog-style alerts (staleness, drawdown, kill switch) |
| `/readiness` | Submission/competition readiness checklist |
| `/events` | Raw append-only event log |
| `/proof` | BNB identity + on-chain proof commitments |
| `/cockpit` | Aggregated dashboard summary |
| `/report`, `/report/markdown` | Run report as JSON / markdown |
| `/export/submission.md` | Submission writeup as markdown |
| `/policy` | Compiled risk policy + hash |
| `/policy/compile` | Compile an NL mandate into a validated policy + hash |
| `/universe` | Eligible-asset universe |
| `/config` | Config inventory |
| `/ops` | Operational status |
| `/metrics` | Metrics summary (JSON) |
| `/assets` | Per-asset feature scores + eligibility |
| `/indicators` | Technical indicators over a deterministic series |
| `/trending` | CMC trending tokens view |
| `/history` | NAV / equity history series |
| `/backtest` | Run the live strategy over a synthetic path |
| `/walkforward` | Walk-forward analysis across windows |
| `/sweep` | Parameter / sentiment sweep |
| `/optimize` | Portfolio-optimizer weights for the current target |
| `/experiments` | Saved backtest experiments + metrics |
| `/skill` | Packaged CMC Skill descriptor |
| `/compete` | Competition contract + registration status |

## Dashboard page inventory (`dashboard/`)

The dashboard is **read-only** — it has no trading path and only renders
`guardrail-api` responses. 64 pages (`dashboard/src/app/*`, routed via
`components/Layout.tsx`):

| Page | Page | Page |
|------|------|------|
| `/` (Cockpit) | `/portfolio` | `/assets` |
| `/indicators` | `/trending` | `/optimizer` |
| `/equity` | `/trades` | `/signals` |
| `/backtest` | `/lab` | `/walkforward` |
| `/sweep` | `/research` | `/skill` |
| `/experiments` | `/compile` | `/risk` |
| `/alerts` | `/readiness` | `/events` |
| `/observability` | `/policy` | `/universe` |
| `/config` | `/ops` | `/proof` |
| `/compete` | `/reports` | |

## Client SDKs, OpenAPI, and deploy surfaces

Read-only consumers of `guardrail-api`, plus the deployment surface:

| Surface | Location | Notes |
|---------|----------|-------|
| TypeScript SDK | `clients/typescript` (`@guardrail/client`) | Typed, dependency-free; Node 18+ and browser via global `fetch`. Read-only. |
| Python SDK | `clients/python` (`guardrail_client`) | Typed read-only client over the same API; ships examples. |
| OpenAPI spec | `docs/api/openapi.yaml` | OpenAPI 3.1, hand-kept in sync with `server.rs`; tags: ops, portfolio, reports, config, analytics. |
| Kubernetes | `deploy/k8s` | Kustomize manifests: namespace, core deployment, services, dashboard. |
| Container stack | `docker-compose.yml` | Full stack (agent, API, exporter, dashboard, Prometheus/Grafana). |

## Trust boundaries

- **LLM is advisory only.** `llm-interface::authorize` is a single choke point;
  the model may translate mandates, explain decisions, and summarize reports,
  but may never authorize swaps, override risk, edit live policy, or bypass the
  allowlist. Rust validation (`policy-compiler::validate_policy`) is
  authoritative.
- **Python is analytics-only**, and the **dashboard and client SDKs are
  read-only**; none has a trading path or any route to TWAK.
- **Risk engine is the only gate** and **TWAK is the sole executor** — both
  enforced structurally (see the two sections above).
- **Policy binds the runtime via hash.** A `RiskPolicy` is compiled, validated,
  and SHA-256 hashed; the hash is the on-chain-publishable fingerprint of
  exactly what governs the agent.
