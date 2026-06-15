# Submission Checklist — Track 1

Maps each Track 1 requirement to where it is satisfied in this repo, with the
real files and commands that prove it.

## Requirement → implementation map

| Requirement | Where it lives | How to verify |
|---|---|---|
| **CMC market data** | `crates/cmc-client` (`CmcDataSource` trait, `rest.rs`, `mcp.rs`, `mock.rs`); normalized by `crates/market-data`. Paper mode uses the deterministic mock (`configs/paper.toml` → `cmc.use_mock = true`). | CMC-derived snapshot drives every cycle; visible as `MarketSnapshotReceived` / `RegimeClassified` events in the log. |
| **TWAK execution** | `crates/twak-client` (`TwakExecutor` trait, `swap.rs`, `quote.rs`, `mock.rs`, `rest.rs`, `mcp.rs`). `configs/*.toml` → `twak.mode`, `twak.quote_before_swap`. | An order reaches TWAK only after risk approval; `guardrail-replay -- trades` lists confirmed swaps. |
| **On-chain registration** | `crates/bnb-agent` (`identity.rs`, `metadata.rs`, `erc8004.rs`, `erc8183.rs`, `proof.rs`, `registration.rs`); `crates/twak-client/src/competition.rs`; `COMPETITION_CONTRACT` in `crates/common/src/constants.rs`; `scripts/register_agent.sh`. Gated by `twak.competition_register_enabled`. | `cargo run -p guardrail-cli -- identity` and `-- register`; on register, an `AgentStarted`/`TxConfirmed` event carries `competition_tx`, surfaced at `GET /proof`. |
| **Risk policy** | `crates/policy-compiler` (NL/JSON → validated `RiskPolicy` + SHA-256 hash); `crates/risk-engine` (the gate); policies in `configs/risk_policy.paper.json`, `configs/risk_policy.production.json`, schema `configs/risk_policy.schema.json`. | `cargo run -p guardrail-cli -- policy compile "<mandate>"` prints policy + hash; `GET /policy` shows the active policy. |
| **Kill switch** | `crates/risk-engine/src/kill_switch.rs` (`should_trigger`); enforced in `crates/agent-runtime/src/runtime.rs` (engages at `kill_switch_drawdown_pct`, stays engaged, halts trading). Manual: `guardrail-cli kill-switch`, `scripts/kill_switch.sh`. | `KillSwitchTriggered` event in the log; `guardrail_kill_switch` gauge flips to `1`; `/alerts` raises it. |
| **Minimum daily trade activity** | `daily_trade_requirement` in the policy (`min_trades_per_day`, `max_heartbeat_trade_pct`); heartbeat injected in `crates/agent-runtime/src/runtime.rs` when a cycle is idle, routed through the same risk gate. | `DailyTradeRequirementSatisfied` events; `guardrail_daily_trade_satisfied_total` gauge. |
| **Eligible-asset allowlist** | `configs/eligible_assets.bsc.json` (USDT, WBNB, CAKE on chain 56) loaded via `market-data::Universe`; `allowed_assets` + `allowed_chains` in the policy enforced by `risk-engine`. | `GET /universe`; risk engine rejects any non-allowlisted symbol or chain. |
| **Read-only dashboard** | `dashboard/` (Next.js) reads from `guardrail-api` only; `apps/guardrail-api` is a read-only axum service over the event store — no trading path. Safety invariants enumerated at `GET /ops`. | Dashboard and API cannot call TWAK; `/ops` lists "API is read-only", "Dashboard cannot call TWAK". |
| **Proof artifacts / hashes** | `policy_hash` (`policy-compiler`), `agent_id` + ERC records + report hash (`bnb-agent`), append-only SQLite log (`crates/event-store`), `data/run_report.json`, `data/exports/submission.md` via `scripts/export_report.sh`. | See "Proof artifacts" below. |

## Operator surfaces (read-only API, port 8080)

- `/health`, `/cockpit`, `/portfolio`, `/trades`, `/signals`, `/risk` populated.
- `/alerts` includes active alerts, counts, and evaluated input values.
- `/readiness` has no blocking checks before submission.
- `/events`, `/policy`, `/universe`, `/config`, `/ops`, `/proof` populated.
- `/metrics` (and the `guardrail-exporter` `/metrics` on port 9100) return NAV,
  drawdown, report age, kill switch, trade, and event gauges.

## Proof artifacts

- [ ] Policy hash generated — `guardrail-cli policy compile`, embedded in
  `AgentStarted` events, exposed at `GET /policy`.
- [ ] Agent identity + ERC-8004/8183 records — `guardrail-cli identity`.
- [ ] Competition registration target / tx — `guardrail-cli register`,
  `GET /proof` (`registration_tx`).
- [ ] Eligible BSC assets loaded — `configs/eligible_assets.bsc.json`,
  `GET /universe`.
- [ ] CMC data visible in the event log — `guardrail-replay journal`.
- [ ] Risk approval and rejection examples captured — `GET /risk`,
  `guardrail-replay summary` (proposed vs. rejected vs. confirmed).
- [ ] TWAK quote + confirmed transaction captured — `guardrail-replay trades`,
  `GET /trades`.
- [ ] Kill-switch behavior demonstrated — `KillSwitchTriggered` event,
  `guardrail_kill_switch` gauge. See `docs/DEMO_SCRIPT.md` §9 (low-threshold demo).
- [ ] Daily report / run report generated — `data/run_report.json`,
  `GET /report`, `GET /report/markdown`.
- [ ] Submission Markdown generated — `data/exports/submission.md` via
  `scripts/export_report.sh`.
- [ ] Dashboard proof page populated — `/proof`.

## Reference docs

`docs/ARCHITECTURE.md`, `docs/RISK.md`, `docs/STRATEGY.md`,
`docs/CMC_INTEGRATION.md`, `docs/TWAK_INTEGRATION.md`,
`docs/BNB_AGENT_IDENTITY.md`, `docs/EXECUTION.md`,
`docs/BACKTEST_METHODOLOGY.md`, `docs/OBSERVABILITY.md`,
`docs/LIVE_RUNBOOK.md`, `docs/DEMO_SCRIPT.md`.
