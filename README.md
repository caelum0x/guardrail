# Guardrail Alpha

**A Rust-native autonomous trading agent for BNB Smart Chain (chain id `56`).** A
plain-English mandate becomes a hashed, machine-verifiable risk policy; live
CoinMarketCap intelligence drives a regime-routed strategy; and a Rust **risk
engine is the sole gate** between intent and execution. Approved orders are
signed and executed only through the **Trust Wallet Agent Kit (TWAK)** under
self-custody — strategy code has no path to the wallet. The agent carries an
on-chain **BNB identity** (ERC-8004 / ERC-8183) with `policy_hash` / `report_hash`
proof commitments, and every decision is logged to an append-only SQLite event
store, hashed, and replayable. The whole pipeline runs **offline in paper mode**
against deterministic CMC + TWAK mocks — no API keys or chain access required.

## Architecture

Rust live engine (the only thing that trades) → emits an append-only event log +
run report → fanned out to read-only consumers: Python analytics, a Next.js
dashboard, SDKs, an MCP server, a Prometheus exporter, and a terminal cockpit.
Authority flows one way; nothing downstream can reach TWAK.

```
                         +-------------------------------------------------+
   CMC data  ──────────► |              LIVE ENGINE (Rust)                 |
   (REST / MCP / x402 /  |  cmc-client ─► market-data ─► feature-engine    |
    Mock)                |                 ─► strategy-engine               |
                         |                        │ intent                 |
                         |                        ▼                        |
                         |   portfolio ─► RISK-ENGINE  ◄── THE ONLY GATE   |
                         |             (pre_trade + final_quote_check)      |
                         |                        │ ApprovedOrder           |
                         |                        ▼                        |
                         |   BNB identity ◄─ twak-client ── THE ONLY EXEC  |
                         |   (ERC-8004/8183) (signs w/ user keys, x402)     |
                         |                        │ fill ─► reconcile       |
                         +------------------------┬------------------------+
                                                  │ emits (never reads back)
              ┌───────────────────┬──────────────┴───────┬──────────────────┐
              ▼                   ▼                       ▼                  ▼
        event-store         data/run_report.json    guardrail-api      guardrail-
        (SQLite)            (NAV, drawdown,          (70 GET routes,    exporter /
                            kill switch)             read-only)         metrics:9100
                                                          │
        ┌─────────────┬──────────────┬──────────────┬────┴──────┬──────────────┐
        ▼             ▼              ▼              ▼           ▼              ▼
   dashboard     web-lite       TS/Python/Go    clients/mcp  python-lab    guardrail-
   (Next.js,     cockpit        SDKs            (CMC Agent   (analytics)   tui / replay
    read-only)   (single file)  (read-only)     Hub server)               (terminal)
```

- **LLM is advisory only** — it may translate a mandate or explain a decision; it
  can never authorize a swap or edit live policy.
- **Python is analytics-only**; the **dashboard, API, and SDKs are read-only** and
  have no path back into the trading loop.
- `strategy-engine` does not depend on `twak-client` or `execution`, so the type
  system enforces the invariant: executing without a risk approval is a compile
  error, not a runtime hope.

Full crate graph, data/trade/risk/event flow, and trust boundaries:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## What's inside

All counts verified against the repo:

| Surface | Count | Where |
|---|---:|---|
| Rust crates (live engine) | **24** | `crates/` |
| Binaries (apps) | **9** | `apps/` |
| Read-only API routes (`GET`) | **70** | `apps/guardrail-api/src/server.rs` |
| Track-2 strategy skills (registered) | **7** | `skills/INDEX.json` (7 skill dirs on disk) |
| Dashboard pages (Next.js) | **64** | `dashboard/src/app/**/page.tsx` |
| Ecosystem clients | **9** | `clients/` |
| Eligible BSC universe | **20** tokens, all `chain_id 56` | `configs/eligible_assets.bsc.json` |

The **9 binaries** are `guardrail-agent` (the only one that trades),
`guardrail-api`, `guardrail-cli`, `guardrail-tui`, `guardrail-monitor`,
`guardrail-exporter`, `guardrail-replay`, `guardrail-sim`, and `guardrail-doctor`.
The **9 clients** are `typescript` (`@guardrail/client`), `python`
(`guardrail_client`), `go` (read-only SDK + `guardrailctl`), `mcp` (CMC Agent Hub
server), `langchain`, `postman`, `proof-verifier` (clean-room verifier),
`web-lite` (cockpit), and `examples`.

## Quickstart

```bash
# 1. Build the Rust workspace
cargo build

# 2. Full offline end-to-end demo: paper run + API on :8080 + web-lite cockpit
scripts/demo.sh

# 3. Capture a paper run + proof artifacts (run report, submission.md, verifier)
scripts/capture_submission.sh
```

Run the pieces individually (all offline, paper mode):

```bash
# Paper trading agent — bounded run, writes SQLite events + run report
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml

# Read-only HTTP API on :8080
cargo run -p guardrail-api

# Next.js dashboard on :3000
cd dashboard && pnpm install && pnpm dev

# Zero-build web-lite cockpit (open in a browser, wire to the API)
open clients/web-lite/index.html        # then point it at http://localhost:8080

# Terminal cockpit, audit replay, walk-forward analysis
cargo run -p guardrail-tui
cargo run -p guardrail-replay -- journal
cargo run -p guardrail-cli -- walk-forward --config configs/paper.toml
```

`make setup`, `make paper`, `make api`, `make dashboard`, `make exporter`, and
`make replay` wrap the common flows. `scripts/guardrail.sh up` is the unified
operator front door (subcommands: `up`, `cockpit`, `analyze`, `scenarios`,
`verify`, `alerts`, `skills`, `new-skill`). `docker compose up --build` brings up
the full stack; `deploy/k8s` carries Kustomize manifests and `deploy/helm` a Helm
chart.

A real **live** run uses `scripts/compete.sh` and requires `CMC_API_KEY`,
`TWAK_BASE_URL`, and `BSC_RPC_URL`; it stays on deterministic mocks if any are
absent. See [docs/LIVE_RUNBOOK.md](docs/LIVE_RUNBOOK.md). Longer step-by-step:
[docs/QUICKSTART.md](docs/QUICKSTART.md).

## Capabilities

| Capability | What it does |
|---|---|
| **Autonomous trading** | Unattended Rust loop (market → regime → risk → TWAK → reconcile → log); the risk engine is the sole gate; dual risk check + kill switch; on-chain registration; 20-asset BSC universe; explainable via the decision journal. In live mode the agent refuses to start on mock data. |
| **Strategy skills** | 7 advisory regime-aware strategy skills (`skills/INDEX.json`) + a regime ensemble meta-allocator (`GET /ensemble`, live in `crates/strategy-ensemble`) + a skill authoring kit (`skills/_template/`, `scripts/new_skill.sh`, `scripts/lint_skills.sh`). |
| **Self-custody execution** | TWAK is the sole execution layer; self-custody is type-enforced; Mock / REST / MCP / CLI transports (20s timeouts); x402 signing; narrated self-custody demo (`scripts/self_custody_demo.sh`). |
| **CMC market intelligence** | All 7 CMC data methods (`crates/cmc-client`), an MCP server + a verifiable CMC data→capability lineage (`GET /cmc/capabilities`, [docs/CMC_AGENT_HUB.md](docs/CMC_AGENT_HUB.md)), x402 pay-and-retry, and a packaged CMC Skill. |
| **Verifiable identity** | ERC-8004 / ERC-8183 identity records, deterministic agent / registration ids, `policy_hash` + `report_hash` commitments with BscScan links, an independent stdlib-only verifier (`clients/proof-verifier`), and read-only on-chain verification (`crates/chain-verifier`). |

## Docs & links

- **[docs/INDEX.md](docs/INDEX.md)** — master table of contents for all documentation.
- **[docs/PRODUCT_OVERVIEW.md](docs/PRODUCT_OVERVIEW.md)** — the high-level product tour and system map.
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — crate graph, flows, and trust boundaries.
- **[docs/LIVE_RUNBOOK.md](docs/LIVE_RUNBOOK.md)** — taking the agent live (real keys, go-live).
- **[PLAN_V2.md](PLAN_V2.md)** — expansion roadmap (offline-safe).
- Earlier competition writeups are archived under **[docs/archive/hackathon/](docs/archive/hackathon/)**.
- **Dashboard (Vercel):** the read-only Next.js dashboard auto-deploys to Vercel on every push — see [docs/VERCEL_DEPLOY.md](docs/VERCEL_DEPLOY.md).

## Submission proof

Everything tying the running agent to its commitments is deterministic and
inspectable offline:

```bash
cargo run -p guardrail-cli -- policy compile "<mandate>"   # validated policy + SHA-256 hash
cargo run -p guardrail-cli -- identity                     # agent id, ERC-8004 record, proof hashes
cargo run -p guardrail-replay -- summary                   # proposed vs rejected vs confirmed
bash scripts/verify_proof.sh                               # independent clean-room proof check
```

The append-only event log (`data/guardrail_alpha.db`) and run report
(`data/run_report.json`) are the book of record; `./scripts/export_report.sh`
writes `data/exports/submission.md`. Copy-pasteable walkthrough:
[docs/JUDGE_DEMO.md](docs/JUDGE_DEMO.md) and
[docs/DEMO_SCRIPT.md](docs/DEMO_SCRIPT.md); Track 1 requirement map:
[docs/SUBMISSION_CHECKLIST.md](docs/SUBMISSION_CHECKLIST.md).
