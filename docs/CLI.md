# Guardrail CLI Reference

`guardrail-cli` is the admin / developer command-line surface for Guardrail
Alpha. It is **read-only and offline-safe**: every subcommand runs in paper mode
against deterministic mocks and the agent's persisted state — none of them holds
keys, signs transactions, or mutates the live book. (The lone exception,
`kill-switch`, only *prints* a trigger line.)

Run any command from the repository root:

```bash
cargo run -p guardrail-cli -- <subcommand> [--flags]
cargo run -p guardrail-cli -- --help          # list every subcommand
cargo run -p guardrail-cli -- <subcommand> --help
```

## How the CLI is organized

Argument parsing and dispatch live in
[`apps/guardrail-cli/src/main.rs`](../apps/guardrail-cli/src/main.rs). Every
`run_*` implementation is delegated to a domain-grouped module under
[`apps/guardrail-cli/src/commands/`](../apps/guardrail-cli/src/commands/), wired
through [`commands/mod.rs`](../apps/guardrail-cli/src/commands/mod.rs):

| Module | File | Command group |
|---|---|---|
| `backtest` | `commands/backtest.rs` | Strategy backtesting & regime/funding analytics |
| `market` | `commands/market.rs` | Market data, quotes, indicators, liquidity |
| `portfolio` | `commands/portfolio.rs` | Budget, drift, mandates, rebalance, scenarios |
| `identity` | `commands/identity.rs` | BNB identity, registration, wallet controls, heartbeat |
| `reporting` | `commands/reporting.rs` | Run reports, submission, prizes, audit manifest |
| `experiment` | `commands/experiment.rs` | Named, persisted backtest experiments |
| `agent_surface` | `commands/agent_surface.rs` | Agent card / services / scorecard / SDK catalog |
| `commerce` | `commands/commerce.rs` | BNB SDK map, ERC-8183 commerce, x402 signing policy |

There are **40 top-level subcommands** (defined in the `Commands` enum in
`main.rs`), two of which — `policy` and `experiment` — have their own nested
subcommands. Common flags: most analytics commands accept `--config`
(default `configs/paper.toml`), `--steps`, and `--preset`
(default `balanced`, see `configs/strategy_presets.json`).

---

## backtest — strategy backtesting & analytics (`commands/backtest.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `backtest` | Run a backtest of the live strategy over a synthetic market path. | `cargo run -p guardrail-cli -- backtest --config configs/paper.toml --steps 60 --preset balanced` |
| `compare` | Backtest all strategy presets side by side and print a comparison table. | `cargo run -p guardrail-cli -- compare --steps 60 --fear-greed 60` |
| `score` | Show the current market regime and asset alpha scores. | `cargo run -p guardrail-cli -- score --config configs/paper.toml` |
| `walk-forward` | Run a walk-forward analysis across sentiment-driven windows. | `cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30 --preset balanced` |
| `regime` | Classify the current market regime and show its sizing exposure. | `cargo run -p guardrail-cli -- regime --config configs/paper.toml` |
| `funding` | Print a per-asset funding-rate-proxy table over a synthetic snapshot. | `cargo run -p guardrail-cli -- funding --steps 48` |

---

## market — market data & execution analytics (`commands/market.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `quote` | Compute a swap quote (price impact + slippage) for a notional. | `cargo run -p guardrail-cli -- quote --from USDT --to WBNB --amount 1000` |
| `markets` | Print a live market table for the eligible universe via the CMC data path. | `cargo run -p guardrail-cli -- markets --config configs/paper.toml` |
| `watchlist` | Rank enabled assets by current attention needs. | `cargo run -p guardrail-cli -- watchlist --limit 12` |
| `liquidity` | Show liquidity capacity and pool usage for eligible assets. | `cargo run -p guardrail-cli -- liquidity --policy configs/liquidity/liquidity_policy.json --limit 12` |
| `costs` | Estimate gas and slippage cost for configured TWAK routes. | `cargo run -p guardrail-cli -- costs --config configs/costs/bsc_execution_costs.json --amount-usd 500` |
| `indicators` | Compute classic technical indicators over a deterministic price series. | `cargo run -p guardrail-cli -- indicators --symbol WBNB --steps 48` |

---

## portfolio — budget, drift, rebalance, scenarios (`commands/portfolio.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `budget` | Show the daily execution budget and gas-runway status. | `cargo run -p guardrail-cli -- budget --config configs/budgets/trading_budget.json` |
| `drift` | Compare current report weights with a fresh strategy target. | `cargo run -p guardrail-cli -- drift --policy configs/drift/drift_policy.json` |
| `mandates` | Compile configured natural-language mandates into policy hashes. | `cargo run -p guardrail-cli -- mandates --config configs/mandates/strategy_mandates.json` |
| `rebalance` | Preview target weights and trade intents without executing anything. | `cargo run -p guardrail-cli -- rebalance --config configs/paper.toml --report data/run_report.json --preset balanced` |
| `exposure` | Show current category exposure from the latest run report. | `cargo run -p guardrail-cli -- exposure --report data/run_report.json` |
| `playbook` | Select the current operator playbook from run state. | `cargo run -p guardrail-cli -- playbook --report data/run_report.json --playbooks configs/playbooks/operator_actions.json` |
| `scenarios` | Apply configured market stress scenarios to the current report. | `cargo run -p guardrail-cli -- scenarios --report data/run_report.json --scenarios configs/scenarios/market_stress.json` |

---

## identity — BNB identity, registration, self-custody (`commands/identity.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `register` | Register the agent for the competition through TWAK (self-custody). | `cargo run -p guardrail-cli -- register --transport mock` |
| `identity` | Print the agent's BNB identity and proof commitments as JSON. | `cargo run -p guardrail-cli -- identity --config configs/paper.toml` |
| `wallet-controls` | Show self-custody wallet and spender control status. | `cargo run -p guardrail-cli -- wallet-controls --config configs/wallet/wallet_controls.json` |
| `exit-triggers` | Evaluate current positions against configured exit triggers. | `cargo run -p guardrail-cli -- exit-triggers --policy configs/exits/exit_policy.json` |
| `heartbeat` | Show Track-1 daily-trade heartbeat status and the planned tiny order. | `cargo run -p guardrail-cli -- heartbeat --config configs/heartbeat/daily_trade.json` |
| `kill-switch` | Emit (print) a kill-switch trigger line. | `cargo run -p guardrail-cli -- kill-switch --reason "demo"` |

> Registration is offline-safe by default (`--transport mock`); `rest`, `mcp`,
> and `cli` transports require a `--base-url` (or `TWAK_BASE_URL`). The
> `kill-switch` command only prints a trigger; the enforcing kill switch lives in
> `crates/risk-engine`.

---

## reporting — run reports, submission, prizes (`commands/reporting.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `report` | Render an offline Markdown run report from the agent's persisted state. | `cargo run -p guardrail-cli -- report --report data/run_report.json` |
| `submission` | Print a concise DoraHacks submission summary from the latest run. | `cargo run -p guardrail-cli -- submission --report data/run_report.json` |
| `briefing` | Print judge/operator briefing claims and demo commands. | `cargo run -p guardrail-cli -- briefing --report data/run_report.json --config configs/briefings/submission_briefing.json` |
| `prizes` | Show the hackathon prize/category evidence map. | `cargo run -p guardrail-cli -- prizes --config configs/submission/prize_map.json --report data/run_report.json` |
| `audit-manifest` | Inventory submission artifacts and declared operator routes. | `cargo run -p guardrail-cli -- audit-manifest --config configs/audit/export_manifest.json` |

---

## experiment — named, persisted backtests (`commands/experiment.rs`)

`experiment` has nested subcommands; results are saved under `data/experiments/`.

| Subcommand | What it does | Example |
|---|---|---|
| `experiment run` | Run a backtest and persist it as a named experiment. | `cargo run -p guardrail-cli -- experiment run --tag baseline --config configs/paper.toml --steps 60 --fear-greed 60 --preset balanced` |
| `experiment list` | List all saved experiments with their key metrics. | `cargo run -p guardrail-cli -- experiment list` |
| `experiment compare` | Print a Markdown table comparing all saved experiments. | `cargo run -p guardrail-cli -- experiment compare` |

---

## agent_surface — agent card / services / scorecard (`commands/agent_surface.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `scorecard` | Show the judge-facing weighted submission scorecard. | `cargo run -p guardrail-cli -- scorecard --config configs/submission/scorecard.json` |
| `sdk-catalog` | Inspect the product-owned BNB Agent SDK integration tree. | `cargo run -p guardrail-cli -- sdk-catalog` |
| `agent-services` | Show ERC-8183 provider service offerings backed by Guardrail routes. | `cargo run -p guardrail-cli -- agent-services --config configs/bnb/agent_services.json` |
| `agent-card` | Render the ERC-8004-style Guardrail agent card. | `cargo run -p guardrail-cli -- agent-card --config configs/bnb/agent_card.json` |
| `job-simulator` | Simulate an ERC-8183 job lifecycle against a Guardrail service. | `cargo run -p guardrail-cli -- job-simulator --config configs/bnb/job_simulator.json` |

---

## commerce — BNB SDK, ERC-8183 commerce, x402 (`commands/commerce.rs`)

| Subcommand | What it does | Example |
|---|---|---|
| `bnb-sdk` | Show the BNB Agent SDK module and contract-mapping evidence. | `cargo run -p guardrail-cli -- bnb-sdk --config configs/bnb/bnb_agent_sdk_map.json` |
| `commerce` | Show the ERC-8183 commerce/provider readiness mapping. | `cargo run -p guardrail-cli -- commerce --config configs/bnb/erc8183_commerce.json` |
| `signing-policy` | Show the x402 and EIP-712 signing-policy controls. | `cargo run -p guardrail-cli -- signing-policy --config configs/x402/signing_policy.json` |

---

## policy — policy hashing & mandate compilation (handled in `main.rs`)

`policy` has nested subcommands for on-chain proof workflows.

| Subcommand | What it does | Example |
|---|---|---|
| `policy hash <path>` | Hash a policy JSON file (SHA-256) for on-chain proof. | `cargo run -p guardrail-cli -- policy hash configs/risk_policy.paper.json` |
| `policy compile <mandate>` | Compile a natural-language mandate into a validated policy + hash. | `cargo run -p guardrail-cli -- policy compile "balanced risk, max 20% per name"` |

---

## See also

- [API.md](API.md) — the read-only HTTP API that mirrors much of this surface.
- [api/openapi.yaml](api/openapi.yaml) — the machine-readable OpenAPI 3.1 spec.
- [OPERATIONS.md](OPERATIONS.md) — operator runbook tying the tooling together.
- [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) — where the CLI fits in the system.
