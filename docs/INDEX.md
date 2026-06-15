# Documentation Index

This is the master table of contents for all Guardrail Alpha documentation. Every
link below points to a real file in this repository; entries are grouped by purpose
so a judge, operator, or engineer can find the right document quickly. Guardrail
Alpha is a Rust-first autonomous trading agent for BNB Smart Chain whose **risk
engine is the sole execution gate** and whose execution flows exclusively through
the Trust Wallet Agent Kit (TWAK). Everything here is offline-safe: the demos and
verification commands run in paper mode against deterministic mocks, with no API
keys or chain access required.

New here? Begin with [Start Here](#start-here), then read [Architecture &
Design](#architecture--design) for the why, and use [Reference](#reference) for the
machine-readable contract.

## Start Here

| Document | Description |
|---|---|
| [PITCH.md](PITCH.md) | The judge-facing pitch: the one invariant and why it matters. |
| [JUDGE_DEMO.md](JUDGE_DEMO.md) | Tight 3–5 minute copy-pasteable live walkthrough for judges. |
| [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) | What Guardrail Alpha is and the non-negotiable risk-gate invariant. |
| [FEATURE_MATRIX.md](FEATURE_MATRIX.md) | Capability matrix mapping each feature to its component and entrypoint. |
| [WHATS_NEW.md](WHATS_NEW.md) | Changelog of the most recently shipped surfaces, grouped by prize lane. |
| [../PLAN_V2.md](../PLAN_V2.md) | Phase-2 expansion roadmap: net-new folders/files deepening each prize lane (offline-safe). |
| [WASM_NOTE.md](WASM_NOTE.md) | Honest record of why in-browser-WASM real-engine backtesting (PLAN_V2 Track B) is deferred and how `/lab` covers it server-side. |
| [DEMO_SCRIPT.md](DEMO_SCRIPT.md) | The exhaustive, literal copy-pasteable demo walkthrough. |
| [HACKATHON.md](HACKATHON.md) | One-page map from Track 1 + special-prize criteria to code. |
| [PRIZE_MAP.md](PRIZE_MAP.md) | Evidence table mapping every targeted prize to concrete code. |

## Architecture & Design

| Document | Description |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System overview: crates, binaries, dashboard, SDKs, and the crate graph. |
| [adr/README.md](adr/README.md) | Index of all Architecture Decision Records. |
| [adr/0001-rust-native-engine.md](adr/0001-rust-native-engine.md) | ADR 0001 — Rust-native live engine; Python for analytics; TS for the dashboard. |
| [adr/0002-risk-engine-is-the-only-gate.md](adr/0002-risk-engine-is-the-only-gate.md) | ADR 0002 — The risk engine is the only gate to execution. |
| [adr/0003-twak-only-execution.md](adr/0003-twak-only-execution.md) | ADR 0003 — All execution flows through TWAK (self-custody). |
| [adr/0004-sqlite-append-only-event-log.md](adr/0004-sqlite-append-only-event-log.md) | ADR 0004 — Append-only SQLite event log as the book of record. |
| [adr/0005-deterministic-mocks-for-paper.md](adr/0005-deterministic-mocks-for-paper.md) | ADR 0005 — Deterministic mocks for paper & backtest. |
| [adr/0006-ensemble-meta-allocator.md](adr/0006-ensemble-meta-allocator.md) | ADR 0006 — Ensemble meta-allocator above the strategy skills. |
| [adr/0007-decision-journal-explainability.md](adr/0007-decision-journal-explainability.md) | ADR 0007 — Decision journal as the explainability projection. |
| [adr/0008-skill-authoring-kit.md](adr/0008-skill-authoring-kit.md) | ADR 0008 — Skill authoring kit: template + validator contract. |
| [adr/0009-alert-relay-out-of-process.md](adr/0009-alert-relay-out-of-process.md) | ADR 0009 — Alert relay as an out-of-process, read-only API consumer. |
| [STRATEGY.md](STRATEGY.md) | The strategy engine pipeline: snapshot → features → regime → orders. |
| [RISK.md](RISK.md) | The risk engine: the only gate, its checks, clipping, and rejection reasons. |
| [EXECUTION.md](EXECUTION.md) | How an approved order becomes a TWAK swap and is reconciled. |
| [ENSEMBLE.md](ENSEMBLE.md) | The regime-routed meta-allocator above the Track-2 strategy skills. |
| [EXPLAINABILITY.md](EXPLAINABILITY.md) | Verifiable autonomy: reconstructing why the agent acted from a tamper-evident record. |

## Integrations

| Document | Description |
|---|---|
| [CMC_INTEGRATION.md](CMC_INTEGRATION.md) | The `cmc-client` crate: the CoinMarketCap data-in layer. |
| [TWAK_INTEGRATION.md](TWAK_INTEGRATION.md) | The `twak-client` crate: the thin TWAK boundary and sole execution layer. |
| [BNB_AGENT_IDENTITY.md](BNB_AGENT_IDENTITY.md) | The `bnb-agent` crate: identity, registry records, and proof hashes. |
| [PROOF_VERIFICATION.md](PROOF_VERIFICATION.md) | How any third party independently verifies the agent's identity offline. |
| [MCP_HUB.md](MCP_HUB.md) | The Guardrail MCP server and how to register it with an MCP host (e.g. CMC Agent Hub). |
| [SELF_CUSTODY.md](SELF_CUSTODY.md) | How self-custody is enforced in code and mapped to the Track 1 penalty ladder. |
| [API_CLIENTS.md](API_CLIENTS.md) | Index of every client option for the read-only Guardrail API, with a chooser table. |
| [ALERTING.md](ALERTING.md) | How operator alerts are surfaced and relayed to human channels. |
| [REALTIME.md](REALTIME.md) | The SSE `/stream` route and the dashboard `/live` page for real-time telemetry. |
| [DASHBOARD.md](DASHBOARD.md) | The read-only Next.js cockpit pages, including `/lab`, `/ensemble`, `/journal`, `/live`, `/skills`, and `/proof`. |
| [SERVICES.md](SERVICES.md) | The read-only `services/` companion tree (control-bot, gateway, report-publisher). |

## Track 2

| Document | Description |
|---|---|
| [TRACK2.md](TRACK2.md) | Track 2 submission: the `regime-routed-bsc-alpha` strategy skill. |
| [SKILL_AUTHORING.md](SKILL_AUTHORING.md) | How to author a new Track-2 strategy skill: layout, contract, helpers. |
| [SKILLS_MARKETPLACE.md](SKILLS_MARKETPLACE.md) | The `skill-loader` crate, `/skills/{id}` API routes, and the dashboard marketplace. |
| [../skills/README.md](../skills/README.md) | The `skills/` catalog: all Track-2 strategy skills and the ensemble. |

## Operations

| Document | Description |
|---|---|
| [OPERATIONS.md](OPERATIONS.md) | Operator guide tying the tooling together (offline-safe, paper mode). |
| [DEPLOYMENT.md](DEPLOYMENT.md) | The four supported deployment paths, from local stack to Kubernetes. |
| [VERCEL_DEPLOY.md](VERCEL_DEPLOY.md) | Deploying the read-only Next.js dashboard to Vercel (auto-deploy on push). |
| [LIVE_RUNBOOK.md](LIVE_RUNBOOK.md) | Runbook for taking the agent live during competition week. |
| [OBSERVABILITY.md](OBSERVABILITY.md) | Prometheus exporter, alert rules, Grafana dashboards, and the watchdog. |
| [SCENARIOS.md](SCENARIOS.md) | The stress scenario library and the guardrail response each one triggers. |
| [BACKTEST_METHODOLOGY.md](BACKTEST_METHODOLOGY.md) | How the backtester validates live trading logic on a synthetic path. |
| [TWAK_SELF_CUSTODY_DEMO.md](TWAK_SELF_CUSTODY_DEMO.md) | Narrated offline walkthrough of the TWAK self-custody flow. |

## Reference

| Document | Description |
|---|---|
| [API.md](API.md) | Human-readable index of the read-only API routes, with sample responses for the headline ones. |
| [api/openapi.yaml](api/openapi.yaml) | OpenAPI 3.1 spec covering the read-only API routes, including `/snapshots`. |
| [api/README.md](api/README.md) | Guide to the OpenAPI spec and the read-only API surface. |
| [CLI.md](CLI.md) | Reference for every `guardrail-cli` subcommand, grouped by the `commands/` modules, with example invocations. |
| [SUBMISSION_CHECKLIST.md](SUBMISSION_CHECKLIST.md) | Track 1 requirements mapped to implementing files and verification commands. |
