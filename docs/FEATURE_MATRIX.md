# Feature Matrix

A comprehensive capability matrix for Guardrail Alpha. Each capability is mapped
to the component(s) that implement it, a real entrypoint/command or path, and a
status. Capabilities are grouped by lane: **Track 1** (autonomous trading),
**Track 2** (skills), **CMC Agent Hub** (MCP), **BNB SDK** (identity),
**TWAK** (self-custody), and **Ops / Observability**.

Every command and path cited below is real and was verified against the repo.
All commands run from the repository root and are **offline-safe** (paper mode,
deterministic mocks, no API keys or chain access). Companion docs:
[PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md), [PRIZE_MAP.md](PRIZE_MAP.md),
[ARCHITECTURE.md](ARCHITECTURE.md), [OPERATIONS.md](OPERATIONS.md).

**Status legend:** Live = wired into the running engine/binaries · Tooling =
script/CLI/SDK surface · Asset = committed config/spec/manifest consumed by code.

---

## Lane: Track 1 — Autonomous trading

| Capability | Component(s) | Entrypoint / path | Status |
|---|---|---|---|
| Autonomous trading loop | `crates/agent-runtime` (`run_cycle`) + `apps/guardrail-agent` | `GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml` | Live |
| Risk engine as sole gate | `crates/risk-engine` (`approve` = `pre_trade` + `final_quote_check`) | `crates/risk-engine/src/checks/` | Live |
| Drawdown throttle + kill switch | `crates/risk-engine` (kill switch, total-drawdown checks) | `cargo run -p guardrail-cli -- kill-switch --reason demo`; `GET /risk` | Live |
| Eligible BSC universe (20 assets) | `configs/eligible_assets.bsc.json`; allowlist check | `cargo run -p guardrail-cli -- markets`; `GET /universe` | Asset |
| Strategy decision (regime + alpha + allocator) | `crates/strategy-engine`, `crates/feature-engine` | `GET /signals`, `/assets`, `/regime` | Live |
| Portfolio optimizer (target weights) | `crates/portfolio-optimizer` | `GET /optimize`; `cargo run -p guardrail-cli -- rebalance` | Live |
| Technical indicators | `crates/indicators` | `cargo run -p guardrail-cli -- indicators`; `GET /indicators` | Live |
| Backtest over production path | `crates/backtester` | `cargo run -p guardrail-cli -- backtest --config configs/paper.toml`; `GET /backtest` | Live |
| Walk-forward analysis | `crates/backtester` + CLI | `cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30`; `GET /walkforward` | Live |
| Scenario / sentiment sweep | `apps/guardrail-sim` | `cargo run -p guardrail-sim`; `GET /sweep` | Live |
| Daily-trade heartbeat | `crates/agent-runtime` (idle-cycle heartbeat) | `GET /heartbeat` | Live |
| Read-only REST API (52 routes) | `apps/guardrail-api` (`src/server.rs::build_app`) | `cargo run -p guardrail-api`; `GET /health`, `/cockpit`, … | Live |
| Admin / dev CLI (40 subcommands) | `apps/guardrail-cli` | `cargo run -p guardrail-cli -- <cmd>` | Tooling |
| Terminal cockpit | `apps/guardrail-tui` | `cargo run -p guardrail-tui` | Live |
| Event-log audit / replay | `apps/guardrail-replay` | `cargo run -p guardrail-replay -- summary` / `journal` / `trades` | Live |
| Preflight doctor | `apps/guardrail-doctor` | `cargo run -p guardrail-doctor` | Live |
| Next.js dashboard (55 pages) | `dashboard/src/app/` | `dashboard/` (read-only) | Live |
| web-lite single-file cockpit | `clients/web-lite/index.html` | `scripts/serve_cockpit.sh` | Tooling |
| Offline E2E demo | `scripts/demo.sh` | `./scripts/demo.sh` | Tooling |
| Judge quickstart (build + agent + API + cockpit) | `scripts/judge_quickstart.sh` | `scripts/judge_quickstart.sh` | Tooling |
| Unified operator front door | `scripts/guardrail.sh` | `scripts/guardrail.sh up` (subcommands: up, cockpit, analyze, scenarios, verify, alerts, skills, new-skill) | Tooling |

## Lane: Track 2 — Strategy skills

| Capability | Component(s) | Entrypoint / path | Status |
|---|---|---|---|
| Skill catalog (4 skills) | `skills/INDEX.json` | `cat skills/INDEX.json`; `GET /skill` | Asset |
| Regime-routed alpha skill | `skills/cmc-regime-routed-alpha/` | `cat skills/cmc-regime-routed-alpha/strategy_spec.yaml` | Asset |
| Funding-rate carry skill | `skills/funding-rate-carry/` | `cat skills/funding-rate-carry/skill.yaml`; `GET /funding` | Asset |
| Mean-reversion chop skill | `skills/mean-reversion-chop/` | `cat skills/mean-reversion-chop/strategy_spec.yaml` | Asset |
| Trend-breakout momentum skill | `skills/trend-breakout-momentum/` | `cat skills/trend-breakout-momentum/strategy_spec.yaml` | Asset |
| Regime ensemble meta-allocator | `skills/ensemble.json` + `python-lab/guardrail_lab/ensemble.py` | `python3 python-lab/analyze.py ensemble --regime chop` | Tooling |
| Ensemble vs single-skill compare | `python-lab/analyze.py` (`ensemble-compare`) | `python3 python-lab/analyze.py ensemble-compare --all` | Tooling |
| Skill authoring scaffold | `skills/_template/` + `scripts/new_skill.sh` | `bash scripts/new_skill.sh demo-skill` | Tooling |
| Skill example linter / validator | `scripts/lint_skills.sh` (`guardrail_lab.skill`) | `bash scripts/lint_skills.sh` | Tooling |
| Skill output-contract tests | `skills/*/tests/*.json` | `cat skills/mean-reversion-chop/tests/test_outputs.json` | Asset |

## Lane: CMC Agent Hub — MCP

| Capability | Component(s) | Entrypoint / path | Status |
|---|---|---|---|
| MCP server (tools + resources + prompts) | `clients/mcp/run.py`, `clients/mcp/guardrail_mcp/` | `python3 clients/mcp/run.py` | Tooling |
| Hub-ready manifest (17 tools, 7 resources, 5 prompts) | `clients/mcp/manifest.json` | `cat clients/mcp/manifest.json` | Asset |
| MCP launch descriptor | `clients/mcp/mcp.json` | `cat clients/mcp/mcp.json` | Asset |
| CMC data source (REST/MCP/x402/Mock) | `crates/cmc-client` (`rest.rs`, `mcp.rs`, `x402.rs`, `mock.rs`) | `GET /quotes`, `/trending`, `/liquidity` | Live |
| x402 pay-and-retry for paid CMC requests | `crates/cmc-client/src/x402.rs` + `retry.rs` | inspect `x402.rs` | Live |
| Packaged CMC Skill descriptor | `skills/cmc-regime-routed-alpha` | `GET /skill` | Asset |
| LangChain tool wrappers | `clients/langchain/` | `clients/langchain/` | Tooling |
| Postman collection | `clients/postman/` | `clients/postman/` | Asset |

## Lane: BNB SDK — identity

| Capability | Component(s) | Entrypoint / path | Status |
|---|---|---|---|
| Agent identity + metadata | `crates/bnb-agent` (`identity.rs`, `metadata.rs`) | `cargo run -p guardrail-cli -- identity --config configs/paper.toml`; `GET /bnb-sdk` | Live |
| ERC-8004 / ERC-8183 records | `crates/bnb-agent/src/{erc8004,erc8183}.rs` | `GET /agent-card`; `cargo run -p guardrail-cli -- agent-card` | Live |
| On-chain proof + commitments | `crates/bnb-agent/src/{proof,report_hash}.rs` | `cargo run -p guardrail-cli -- identity`; `GET /proof` | Live |
| On-chain competition registration | `crates/agent-runtime` (`register_competition`) + CLI | `cargo run -p guardrail-cli -- register`; `GET /compete` | Live |
| Independent clean-room proof verifier | `clients/proof-verifier/verify.py` | `python3 clients/proof-verifier/verify.py --strict`; `bash scripts/verify_proof.sh` | Tooling |
| BNB agent SDK integration | `integrations/bnbagent-sdk/` | `integrations/bnbagent-sdk/` | Tooling |

## Lane: TWAK — self-custody

| Capability | Component(s) | Entrypoint / path | Status |
|---|---|---|---|
| TWAK as sole executor | `crates/twak-client` (`execute_swap(&ApprovedOrder)`) | `cargo run -p guardrail-replay -- trades` | Live |
| Self-custody type enforcement | `crates/twak-client/src/{lib,approvals,swap}.rs` | read `crates/twak-client/src/lib.rs` | Live |
| Execution transports (Mock/REST/MCP/CLI) | `crates/twak-client/src/{mock,rest,mcp,cli}.rs` | inspect transport modules | Live |
| x402 signing | `crates/twak-client/src/x402.rs` | inspect `x402.rs` | Live |
| Example signing policy / envelope | `configs/signing_policy.example.json` | `cat configs/signing_policy.example.json`; `GET /signing-policy` | Asset |
| Narrated self-custody demo | `scripts/self_custody_demo.sh` | `bash scripts/self_custody_demo.sh` | Tooling |
| Wallet controls / commerce surfaces | `apps/guardrail-api` | `GET /wallet-controls`, `/commerce` | Live |

## Lane: Ops / Observability

| Capability | Component(s) | Entrypoint / path | Status |
|---|---|---|---|
| Watchdog monitor → notifier | `apps/guardrail-monitor` + `crates/notifier` | `GUARDRAIL_MONITOR_CHECKS=1 cargo run -p guardrail-monitor` | Live |
| Alert relay to chat sinks (dry-run default) | `integrations/alert-relay/relay.py` | `python3 integrations/alert-relay/relay.py --once --dry-run` | Tooling |
| Prometheus exporter (`:9100`) | `apps/guardrail-exporter` | `cargo run -p guardrail-exporter`; `GET :9100/metrics` | Live |
| Prometheus + Grafana configs | `infra/prometheus/`, `infra/grafana/` | `infra/prometheus/` | Asset |
| Stress scenario library | `configs/scenarios/` (`index.json` + per-scenario) | `bash scripts/run_scenarios.sh`; `GET /scenarios` | Asset |
| Analytics (regime/drawdown/montecarlo/dossier) | `python-lab/analyze.py` + `guardrail_lab/` | `python3 python-lab/analyze.py regime` | Tooling |
| Decision journal projection | `python-lab/analyze.py` (`journal`) | `python3 python-lab/analyze.py journal --out data/journal.md` | Tooling |
| Helm chart | `deploy/helm/guardrail` | `deploy/helm/guardrail/Chart.yaml` | Asset |
| Kubernetes (Kustomize) manifests | `deploy/k8s` | `deploy/k8s/kustomization.yaml` | Asset |
| Full container stack | `docker-compose.yml` + `infra/Dockerfile.*` | `docker compose up` | Asset |
| systemd units | `infra/systemd/` | `infra/systemd/` | Asset |
| Health check / readiness | `apps/guardrail-api` | `GET /health`, `/readiness`; `scripts/healthcheck.sh` | Live |

---

## Read-only client SDKs

| SDK | Package | Path |
|---|---|---|
| TypeScript | `@guardrail/client` | `clients/typescript/` |
| Python | `guardrail_client` | `clients/python/` |
| Go | read-only Go client | `clients/go/` |
| OpenAPI spec | OpenAPI 3.1 | `docs/api/openapi.yaml` |

All SDKs and dashboards are read-only consumers of `guardrail-api`; none has a
trading path or any route to TWAK.

---

## Verified counts

| Quantity | Count | Source |
|---|---|---|
| Rust crates | 19 | `crates/` |
| Rust binaries | 9 | `apps/` |
| API routes (read-only `GET`) | 52 | `apps/guardrail-api/src/server.rs` |
| CLI subcommands | 40 | `apps/guardrail-cli/src/main.rs` |
| Next.js dashboard page routes | 55 | `dashboard/src/app/` |
| Track-2 strategy skills | 4 | `skills/INDEX.json` |
| `analyze.py` subcommands | 7 | `python-lab/analyze.py` |
| MCP tools / resources / prompts | 17 / 7 / 5 | `clients/mcp/manifest.json` |
| `clients/` packages | 9 | `clients/` |
