# Hackathon Index

A one-page map from Track 1 requirements and the special-prize criteria to
exactly where each is implemented in this repo. Every row is grounded in code.

## Track 1 requirements

| Requirement | Where it is implemented | Verify |
|---|---|---|
| **CMC data in** | `crates/cmc-client` — `CmcDataSource` trait with REST / MCP (CMC AI Agent Hub) / x402 / Mock transports; consumed via `market-data::SnapshotBuilder`. Uses quotes, OHLCV, Fear & Greed, DEX liquidity, token security, trending, global. | [CMC_INTEGRATION.md](CMC_INTEGRATION.md) |
| **TWAK execution out** | `crates/twak-client` — `TwakExecutor` is the sole execution layer; quote → risk → execute → reconcile in `agent-runtime/src/runtime.rs::process_order`. | [TWAK_INTEGRATION.md](TWAK_INTEGRATION.md) |
| **BNB AI Agent SDK** | `crates/bnb-agent` — `AgentIdentity`, `AgentMetadata`, ERC-8004/8183 records, registration artifacts, `AgentProof` with BscScan links; wired into the runtime's start/report events. | [BNB_AGENT_IDENTITY.md](BNB_AGENT_IDENTITY.md) |
| **On-chain registration** | `register_competition()` on `TwakExecutor`, called by the runtime when `twak.competition_register_enabled`; contract `0x212c61b9b72c95d95bf29cf032f5e5635629aed5` in `crates/common/src/constants.rs`; CLI `guardrail-cli register`; `twak compete register` / `scripts/register_agent.sh`. | `crates/agent-runtime/src/runtime.rs` (lines ~107–119) |
| **≥1 trade/day** | `policy.daily_trade_requirement { min_trades_per_day: 1 }`; if a cycle trades nothing, the runtime injects a compliant heartbeat (`heartbeat_intent`, capped at `max_heartbeat_trade_pct`) and emits `DailyTradeRequirementSatisfied`. | `configs/risk_policy.*.json`; `agent-runtime/src/runtime.rs` |
| **Eligible 149-token list** | `configs/eligible_assets.bsc.json` — a curated BSC subset of the Track 1 eligible universe (20 tokens, all `chain_id 56`), loaded via `market-data::Universe`. The policy's `allowed_assets` + `allowed_chains` (`[56]`) further constrain the tradeable set; non-eligible trades are rejected by `risk-engine/src/checks/asset_allowlist.rs`. | `GET /universe`; `asset_allowlist::check` |
| **<30% drawdown (DQ)** | Soft limit `max_total_drawdown_pct = 22` (reduce-only throttle) and hard `kill_switch_drawdown_pct = 24` (halts + stays engaged) — both safely under the 30% DQ line. Enforced in `kill_switch::should_trigger` and `checks/total_drawdown.rs`. | `configs/risk_policy.*.json`; `KillSwitchTriggered` event |
| **Hold non-zero in-scope balance** | Paper book seeds `$10,000` in the stable reserve (`PortfolioState::seed_stable`); `min_stable_reserve_pct` keeps a non-zero in-scope (USDT) reserve at all times; `checks/stable_reserve.rs` + `checks/wallet_balance.rs` enforce sufficiency. | `agent-runtime/src/runtime.rs`; `risk-engine` checks |

## Self-custody

Fully self-custodial: the engine only builds intents/approvals; TWAK holds keys
and signs; `execute_swap` requires an engine-minted `ApprovedOrder`, so executing
without risk approval is a compile error. Targets the top band of the
self-custody penalty ladder. See [SELF_CUSTODY.md](SELF_CUSTODY.md) and
[ADR 0003](adr/0003-twak-only-execution.md).

## Special-prize mapping

| Prize | Summary | Detail |
|---|---|---|
| **Best Use of TWAK** | Sole execution layer; Mock/REST/MCP/CLI transports; type-enforced self-custody; autonomous loop with dual risk gate + kill switch; x402 signing; competition registration. Scoring: integration depth (30), self-custody (25), autonomous+guardrails (20), x402 (10), originality (10), demo (5). | [TWAK_INTEGRATION.md](TWAK_INTEGRATION.md) |
| **Best Use of Agent Hub** | All seven CMC data methods; MCP client to the CMC AI Agent Hub; x402 pay-and-retry; packaged CMC Skill `skills/cmc-regime-routed-alpha`. | [CMC_INTEGRATION.md](CMC_INTEGRATION.md) |
| **Best Use of BNB AI Agent SDK** | ERC-8004/8183 identity records, deterministic agent/registration ids, policy_hash + report_hash commitments, BscScan proof links. | [BNB_AGENT_IDENTITY.md](BNB_AGENT_IDENTITY.md) |

## Track 2 — Strategy Skills

The repo also ships a Track-2 entry: a backtestable, regime-routed strategy
authored as an LLM **Skill** (`regime-routed-bsc-alpha`), advisory-only and
validated against the same Rust engine that trades it live.

| Item | Where |
|---|---|
| Submission writeup | [TRACK2.md](TRACK2.md) |
| Skill artifact | [`skills/cmc-regime-routed-alpha/`](../skills/cmc-regime-routed-alpha) (`skill.yaml`, `strategy_spec.yaml`, `prompts/`, `examples/`, `tests/`) |
| Strategy logic | [STRATEGY.md](STRATEGY.md) |
| Backtest tooling | [BACKTEST_METHODOLOGY.md](BACKTEST_METHODOLOGY.md) — `guardrail-cli backtest / walk-forward / sweep`, `guardrail-sim`, `python-lab` experiment tracking |

## Related docs

- [ARCHITECTURE.md](ARCHITECTURE.md) — system overview
- [RISK.md](RISK.md) — risk checks and policy
- [STRATEGY.md](STRATEGY.md) — regime routing and scoring
- [SUBMISSION_CHECKLIST.md](SUBMISSION_CHECKLIST.md) — submission checklist
- [DEMO_SCRIPT.md](DEMO_SCRIPT.md) — demo walkthrough
- [adr/](adr/) — architecture decision records
