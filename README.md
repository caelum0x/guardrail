# Guardrail Alpha

A Rust-native autonomous trading agent for BNB Smart Chain (chain id `56`).

Natural-language mandate in. CMC market intelligence in. A Rust risk engine in
control. TWAK-signed trades out. Every decision logged, hashed, and replayable.

## What it does

Guardrail Alpha turns a plain-English trading mandate into a machine-verifiable
risk policy, then runs an autonomous loop that the policy governs end to end:

```
NL mandate ─▶ compiled & hashed RiskPolicy ─▶ regime-routed strategy
   ─▶ risk gate (the only authority boundary) ─▶ TWAK quote + final risk
   ─▶ TWAK execution ─▶ portfolio reconcile ─▶ SQLite event log + run report
   ─▶ API / dashboard / SDKs / exporter / monitor / TUI
```

- Reads live market, DEX, liquidity, token-security, trending, and Fear & Greed
  sentiment from CoinMarketCap (`cmc-client`); paper mode uses a deterministic
  CMC mock so the whole flow runs offline and reproducibly.
- Compiles a natural-language mandate into a validated `RiskPolicy` and a
  SHA-256 `policy_hash` (`policy-compiler`).
- Classifies the market regime, computes technical indicators (`indicators`),
  scores BSC-eligible assets, and builds a target portfolio with a
  weight optimizer (`feature-engine`, `strategy-engine`, `portfolio-optimizer`).
- Lets an order reach execution only after the Rust risk engine approves it
  twice — pre-trade and again after the quote (`risk-engine`).
- Executes only through the Trust Wallet Agent Kit (`twak-client`); supports
  x402 pay-per-request for premium CMC data, where TWAK (self-custody) signs the
  payment authorization.
- Records every step as an append-only `AgentEvent` in SQLite (`event-store`)
  and writes a `data/run_report.json` snapshot.
- Carries an on-chain agent identity with ERC-8004 / ERC-8183 registry records
  and proof commitments (`bnb-agent`).
- Publishes a read-only Next.js dashboard, two client SDKs, a terminal cockpit,
  and a Prometheus metrics surface; out-of-band alerts are delivered via
  `notifier`.

## Architecture

A Cargo workspace of focused crates plus nine binaries. Dependencies flow one
way; authority never flows back down. The strategy crate has no path to the
executor — `strategy-engine` does not depend on `twak-client` or `execution`, so
the **risk engine is the sole gate** between intent and a swap, and **TWAK is the
sole executor**. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full
crate graph, data/trade/risk/event flow, and trust boundaries.

Trust boundaries: the **LLM is advisory only** (may translate mandates and
explain decisions, never authorize swaps or edit live policy); **Python is
analytics-only**; the **dashboard, API, and SDKs are read-only** and cannot reach
TWAK. The policy binds the runtime via its hash.

## Binaries

All run from the repo root.

| Binary | Purpose | Run command |
|---|---|---|
| `guardrail-agent` | The autonomous trading loop (market → strategy → risk → TWAK → reconcile → log). | `cargo run -p guardrail-agent -- --config configs/paper.toml` |
| `guardrail-api` | Axum read-only HTTP API over the event store and run report (30 routes). | `cargo run -p guardrail-api` (binds `0.0.0.0:8080`) |
| `guardrail-cli` | Dev/admin CLI: compile/hash policy, score, quote, backtest, compare, walk-forward, markets, indicators, experiments, identity, register, kill-switch, report, submission. | `cargo run -p guardrail-cli -- <subcommand>` |
| `guardrail-tui` | Terminal cockpit: polls the run report + event totals and renders to the terminal. | `cargo run -p guardrail-tui` |
| `guardrail-monitor` | Watchdog: raises alerts on staleness, drawdown, kill switch (dispatches via `notifier`). | `cargo run -p guardrail-monitor` |
| `guardrail-exporter` | Prometheus `/metrics` sidecar derived from the event log + run report. | `cargo run -p guardrail-exporter` (binds `0.0.0.0:9100`) |
| `guardrail-replay` | Read-only audit of the SQLite event log (journal / trades / summary / CSV). | `cargo run -p guardrail-replay -- journal` |
| `guardrail-sim` | Sentiment sweep / walk-forward over the real backtest engine. | `cargo run -p guardrail-sim` |
| `guardrail-doctor` | Preflight checks: config load, risk-policy validation, universe, data-dir writability. | `cargo run -p guardrail-doctor` |

## API endpoints (`guardrail-api`, read-only, port 8080)

All routes are side-effect-free `GET`s. 30 in total.

| Endpoint | Returns |
|---|---|
| `GET /health` | Liveness + event-store connectivity. |
| `GET /portfolio` | Latest portfolio reconcile state. |
| `GET /trades` | Confirmed on-chain swaps. |
| `GET /signals` | Latest regime + strategy signals. |
| `GET /risk` | Recent risk decisions (approve / clip / reject). |
| `GET /alerts` | Active alerts, counts, and evaluated input values. |
| `GET /readiness` | Submission readiness checks (blocking vs. non-blocking). |
| `GET /events` | Recent raw `AgentEvent` log. |
| `GET /proof` | Agent id, registration tx, latest report, run report. |
| `GET /cockpit` | Aggregated overview (health, regime, target, kill switch, tx count). |
| `GET /report` | Run report JSON. |
| `GET /report/markdown` | Run report rendered as Markdown. |
| `GET /export/submission.md` | Submission Markdown artifact. |
| `GET /policy` | Active risk policy + hash. |
| `GET /policy/compile` | Compile an NL mandate into a validated policy + hash. |
| `GET /universe` | Eligible BSC asset allowlist. |
| `GET /config` | Config-file inventory. |
| `GET /ops` | Operator commands, HTTP surfaces, safety invariants. |
| `GET /metrics` | NAV, drawdown, report age, kill switch, trade/event gauges. |
| `GET /assets` | Per-asset feature scores and eligibility. |
| `GET /indicators` | Technical indicators over a deterministic price series. |
| `GET /trending` | CMC trending tokens view. |
| `GET /history` | NAV / equity history series. |
| `GET /backtest` | On-demand backtest run. |
| `GET /walkforward` | On-demand walk-forward analysis. |
| `GET /sweep` | Sentiment / parameter sweep. |
| `GET /optimize` | Portfolio-optimizer weights for the current target. |
| `GET /experiments` | Saved backtest experiments and metrics. |
| `GET /skill` | Packaged CMC Skill descriptor. |
| `GET /compete` | Competition contract + registration status. |

## Dashboard pages (Next.js, read-only)

29 pages, all rendering `guardrail-api` responses:

`/` (Cockpit), `/portfolio`, `/assets`, `/indicators`, `/trending`,
`/optimizer`, `/equity`, `/trades`, `/signals`, `/backtest`, `/lab`,
`/walkforward`, `/sweep`, `/research`, `/skill`, `/experiments`, `/compile`,
`/risk`, `/alerts`, `/readiness`, `/events`, `/observability`, `/policy`,
`/universe`, `/config`, `/ops`, `/proof`, `/compete`, `/reports`.

## Client SDKs + OpenAPI

| Surface | Location | Notes |
|---|---|---|
| TypeScript SDK | `clients/typescript` (`@guardrail/client`) | Typed, dependency-free; Node 18+ and browser (global `fetch`). Read-only. |
| Python SDK | `clients/python` (`guardrail_client`) | Typed read-only client over the same API. |
| OpenAPI spec | `docs/api/openapi.yaml` | OpenAPI 3.1, hand-kept in sync with `apps/guardrail-api/src/server.rs`. |

## Quickstart

```bash
# Build the Rust workspace
cargo build

# Paper run — full offline end-to-end demo (deterministic CMC + TWAK mocks)
./scripts/demo.sh

# Live competition launcher (registers via TWAK, runs against production config)
./scripts/compete.sh
```

`scripts/demo.sh` exercises the entire pipeline in one command: doctor preflight,
NL→policy compile, a bounded paper agent run (SQLite events + run report),
replay audit, exporter metrics, backtest / walk-forward / sweep, markets,
identity, and submission report. `scripts/compete.sh` is the live launcher; a
real live run requires `CMC_API_KEY`, `TWAK_BASE_URL`, and `BSC_RPC_URL` (it
stays on deterministic mocks if any are absent). See
[docs/LIVE_RUNBOOK.md](docs/LIVE_RUNBOOK.md).

Individual flows:

```bash
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml
cargo run -p guardrail-api                                   # API on :8080
cd dashboard && pnpm install && pnpm dev                     # dashboard on :3000
cargo run -p guardrail-cli -- walk-forward --config configs/paper.toml
```

`make setup`, `make paper`, `make api`, `make dashboard`, `make exporter`, and
`make replay` wrap the common flows; `docker compose up --build` brings up the
full stack; `deploy/k8s` carries Kustomize manifests.

## Eligible universe

The tradeable set is the BSC subset of the Track 1 eligible universe defined in
`configs/eligible_assets.bsc.json` — **20 tokens, all `chain_id 56`**, all
currently enabled:

`USDT`, `USDC`, `WBNB`, `ETH`, `BTCB`, `CAKE`, `UNI`, `AAVE`, `LINK`, `DOT`,
`ATOM`, `FIL`, `INJ`, `XRP`, `ADA`, `LTC`, `DOGE`, `SHIB`, `TWT`, `AVAX`.

The policy's `allowed_assets` and `allowed_chains` (`[56]`) further constrain the
set; non-eligible trades are rejected by `risk-engine`'s asset allowlist check.
Inspect it live at `GET /universe`.

## Hackathon mapping

Submission targets **Track 1 — Autonomous Trading Agents**, plus three special
prizes:

| Prize | One-line claim |
|---|---|
| **Best Use of TWAK** | TWAK is the sole execution layer; self-custody is type-enforced (executing without a risk approval is a compile error); Mock / REST / MCP / CLI transports, x402 signing, autonomous competition registration. |
| **Best Use of Agent Hub (CoinMarketCap)** | All seven CMC data methods, an MCP client to the CMC AI Agent Hub, x402 pay-and-retry, and a packaged CMC Skill (`skills/cmc-regime-routed-alpha`). |
| **Best Use of BNB AI Agent SDK** | ERC-8004 / ERC-8183 identity records, deterministic agent / registration ids, `policy_hash` + `report_hash` proof commitments with BscScan links. |

Full submission writeup: [SUBMISSION.md](SUBMISSION.md). Requirement-to-code map:
[docs/HACKATHON.md](docs/HACKATHON.md).

## Submission proof

Everything that ties the running agent to its commitments is deterministic and
inspectable:

- **Policy hash** — `cargo run -p guardrail-cli -- policy compile "<mandate>"`
  prints the validated policy and its SHA-256 hash; the same hash is embedded in
  every `AgentStarted` event and exposed at `GET /policy`.
- **Agent identity** — `cargo run -p guardrail-cli -- identity` prints the agent
  id (SHA-256 of name + wallet), wallet, address URL, policy hash, metadata, and
  the ERC-8004 record. Competition target: `cargo run -p guardrail-cli -- register`.
- **Event log** — the append-only SQLite log (`data/guardrail_alpha.db`) is the
  audit trail; inspect it with `cargo run -p guardrail-replay -- journal` /
  `trades` / `summary`, or `GET /events`.
- **Run report** — `data/run_report.json` (NAV, drawdown, positions, kill
  switch, age) feeds `GET /report`, `GET /report/markdown`, and the Prometheus
  exporter. `./scripts/export_report.sh` writes `data/exports/submission.md`.

See [docs/DEMO_SCRIPT.md](docs/DEMO_SCRIPT.md) for a copy-pasteable walkthrough
and [docs/SUBMISSION_CHECKLIST.md](docs/SUBMISSION_CHECKLIST.md) for the Track 1
requirement map.
# guardrail
