# Quickstart

The long-form, step-by-step companion to the [README](../README.md). Everything
here runs **offline in paper mode** against deterministic CMC + TWAK mocks — no
API keys, no chain access. All commands run from the repo root.

## Prerequisites

- **Rust** (stable; pinned in `rust-toolchain.toml`) with `cargo`.
- **Node + pnpm** (only for the Next.js dashboard; the web-lite cockpit needs neither).
- **Python 3** (only for analytics, the proof verifier, and the MCP server).
- Optional: Docker / Docker Compose for the full-stack path.

## 1. Build the workspace

```bash
cargo build
```

This builds all 21 crates and 9 binaries in the Cargo workspace.

## 2. Fastest path — the judge tour

```bash
scripts/judge_quickstart.sh
```

Idempotent and offline. It builds the workspace, runs the paper agent once to
populate `data/` if no run data exists, starts `guardrail-api` on `:8080` and
waits for `/health`, serves the web-lite cockpit wired to the API, and prints a
panel of URLs and curlable endpoints. Background processes are cleaned up on exit.
Flags: `--no-build` (skip cargo build), `--port <n>` (cockpit static port).

## 3. Full end-to-end demo

```bash
scripts/demo.sh
```

Exercises the entire pipeline in one command: doctor preflight → NL→policy
compile → a bounded paper agent run (SQLite events + run report) → replay audit →
exporter metrics → backtest / walk-forward / sweep → markets → identity →
submission report.

## 4. Run the pieces individually

### Paper trading agent

```bash
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml
```

Runs the autonomous loop for a bounded number of cycles: pull market data → mark
portfolio + update drawdown → strategy decides intent → risk gate per order
(pre-trade and post-quote) → TWAK signs/executes → reconcile fills → append every
step to the event store. Writes `data/guardrail_alpha.db` and
`data/run_report.json`.

### Read-only API (`:8080`)

```bash
cargo run -p guardrail-api
```

68 side-effect-free `GET` routes over the event store and run report. Spot-check a
few:

```bash
curl -fsS http://127.0.0.1:8080/health      # liveness + event-store connectivity
curl -fsS http://127.0.0.1:8080/universe    # 20 eligible BSC assets
curl -fsS http://127.0.0.1:8080/proof       # BNB identity + registration proof
curl -fsS http://127.0.0.1:8080/prizes      # the prize evidence map, with live facts
curl -fsS http://127.0.0.1:8080/journal     # decision-journal projection
curl -fsS http://127.0.0.1:8080/ensemble    # regime ensemble meta-allocator
curl -fsS http://127.0.0.1:8080/version     # service version + uptime
```

### Dashboards

```bash
# Next.js dashboard on :3000 (63 read-only pages)
cd dashboard && pnpm install && pnpm dev

# Zero-build web-lite cockpit (single file) — open and point at the API
open clients/web-lite/index.html
```

The Next.js dashboard only renders `guardrail-api` responses and auto-deploys to
Vercel on every push ([docs/VERCEL_DEPLOY.md](VERCEL_DEPLOY.md)). Headline pages:
`/live` (real-time SSE telemetry), `/lab` (server-backed Strategy Lab),
`/ensemble`, `/journal`, `/skills`, and `/proof`.

### Terminal cockpit, replay, analysis

```bash
cargo run -p guardrail-tui                                  # live terminal panels
cargo run -p guardrail-replay -- journal                   # audit the event log
cargo run -p guardrail-replay -- summary                   # proposed vs rejected vs confirmed
cargo run -p guardrail-cli -- walk-forward --config configs/paper.toml
cargo run -p guardrail-exporter                            # Prometheus /metrics on :9100
```

## 5. Make targets

`make setup`, `make paper`, `make api`, `make dashboard`, `make exporter`,
`make replay`, `make backtest`, `make monitor`, `make register`, `make kill`,
`make stack-up`, `make stack-down`, and `make metrics` wrap the common flows.

## 6. Unified operator front door

```bash
scripts/guardrail.sh up          # build + run paper agent + serve API and cockpit
scripts/guardrail.sh cockpit     # terminal cockpit
scripts/guardrail.sh analyze     # python-lab analytics
scripts/guardrail.sh scenarios   # stress scenario library
scripts/guardrail.sh verify      # independent proof verification
scripts/guardrail.sh alerts      # alert relay (dry-run)
scripts/guardrail.sh skills      # list Track-2 skills
scripts/guardrail.sh new-skill   # scaffold a new strategy skill
```

## 7. Full stack (Docker)

```bash
docker compose up --build        # agent + API + exporter + dashboard
```

Kustomize manifests live under `deploy/k8s`; a Helm chart under
`deploy/helm/guardrail`. Per-binary Dockerfiles and Prometheus/Grafana configs
are under `infra/`.

## 8. Prize evidence — verify offline

```bash
bash scripts/lint_skills.sh                              # validate strategy skills
bash scripts/verify_proof.sh                            # independent on-chain proof check
bash scripts/self_custody_demo.sh                       # TWAK self-custody walkthrough
cat clients/mcp/manifest.json                           # MCP tools + resources + prompts
./scripts/export_report.sh                              # writes data/exports/submission.md
```

## 9. Going live (competition week)

```bash
scripts/compete.sh
```

Registers via TWAK and runs against the production config. Requires `CMC_API_KEY`,
`TWAK_BASE_URL`, and `BSC_RPC_URL` (stays on deterministic mocks if any are
absent). Full procedure: [LIVE_RUNBOOK.md](LIVE_RUNBOOK.md).

---

More: [docs/INDEX.md](INDEX.md) (all docs), [docs/PRIZE_MAP.md](PRIZE_MAP.md)
(evidence table), [docs/ARCHITECTURE.md](ARCHITECTURE.md) (crate graph),
[docs/JUDGE_DEMO.md](JUDGE_DEMO.md) (3–5 minute walkthrough).
