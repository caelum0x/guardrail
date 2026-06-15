Absolutely. Here is the **full production-grade folder/file architecture** for the project.

We should structure it as a serious Rust-native autonomous trading system:

```text
Rust = live trading engine
Python = research, backtesting analysis, charts, reports
TypeScript = dashboard only
TWAK = signing + execution
CMC = market/DEX/sentiment data
BNB SDK = agent identity/proof
```

This matches the Track 1 requirement: the agent must read markets via CMC, decide, sign/process transactions via TWAK, trade live on BSC, respect rules, register on-chain, and maintain minimum daily trade activity. 

---

# Project name

Use:

```text
guardrail-alpha/
```

or if you want the Rust-native brand:

```text
iron-alpha/
```

I will use **guardrail-alpha** below.

---

# Full repository architecture

```text
guardrail-alpha/
│
├── README.md
├── LICENSE
├── .gitignore
├── .env.example
├── docker-compose.yml
├── Makefile
├── justfile
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── clippy.toml
├── deny.toml
│
├── configs/
│   ├── production.toml
│   ├── paper.toml
│   ├── backtest.toml
│   ├── risk_policy.production.json
│   ├── risk_policy.paper.json
│   ├── risk_policy.schema.json
│   ├── eligible_assets.bsc.json
│   ├── asset_categories.json
│   ├── strategy_weights.json
│   ├── execution_limits.json
│   └── secrets.example.toml
│
├── crates/
│   ├── common/
│   ├── market-data/
│   ├── cmc-client/
│   ├── feature-engine/
│   ├── strategy-engine/
│   ├── risk-engine/
│   ├── portfolio/
│   ├── execution/
│   ├── twak-client/
│   ├── bnb-agent/
│   ├── event-store/
│   ├── backtester/
│   ├── policy-compiler/
│   ├── llm-interface/
│   ├── observability/
│   └── agent-runtime/
│
├── apps/
│   ├── guardrail-agent/
│   ├── guardrail-api/
│   ├── guardrail-cli/
│   └── guardrail-monitor/
│
├── python-lab/
│   ├── pyproject.toml
│   ├── requirements.txt
│   ├── notebooks/
│   ├── scripts/
│   ├── reports/
│   ├── data/
│   └── guardrail_lab/
│
├── dashboard/
│   ├── package.json
│   ├── pnpm-lock.yaml
│   ├── next.config.ts
│   ├── tsconfig.json
│   ├── src/
│   └── public/
│
├── skills/
│   └── cmc-regime-routed-alpha/
│
├── docs/
│   ├── ARCHITECTURE.md
│   ├── STRATEGY.md
│   ├── RISK.md
│   ├── EXECUTION.md
│   ├── TWAK_INTEGRATION.md
│   ├── CMC_INTEGRATION.md
│   ├── BNB_AGENT_IDENTITY.md
│   ├── LIVE_RUNBOOK.md
│   ├── BACKTEST_METHODOLOGY.md
│   ├── DEMO_SCRIPT.md
│   └── SUBMISSION_CHECKLIST.md
│
├── migrations/
│   ├── 0001_init.sql
│   ├── 0002_market_snapshots.sql
│   ├── 0003_trade_events.sql
│   ├── 0004_risk_events.sql
│   └── 0005_agent_reports.sql
│
├── tests/
│   ├── fixtures/
│   ├── integration/
│   ├── replay/
│   └── smoke/
│
├── scripts/
│   ├── setup.sh
│   ├── register_agent.sh
│   ├── paper_trade.sh
│   ├── live_trade.sh
│   ├── run_backtest.sh
│   ├── export_report.sh
│   ├── healthcheck.sh
│   └── kill_switch.sh
│
├── infra/
│   ├── Dockerfile.agent
│   ├── Dockerfile.api
│   ├── Dockerfile.dashboard
│   ├── prometheus/
│   ├── grafana/
│   └── systemd/
│
└── .github/
    └── workflows/
        ├── rust-ci.yml
        ├── dashboard-ci.yml
        ├── python-ci.yml
        ├── docker-build.yml
        └── security.yml
```

---

# Top-level files

## `README.md`

This is the judge-facing entry point.

```text
README.md
```

Should contain:

```md
# Guardrail Alpha

A Rust-native autonomous trading agent for BSC.

Natural-language mandate in.
CMC market intelligence in.
Rust risk engine in control.
TWAK-signed trades out.

## What it does

- Reads live market, DEX, liquidity, sentiment, and Fear & Greed data from CMC.
- Converts a natural-language strategy into a machine-verifiable risk policy.
- Scores BSC-eligible assets.
- Builds a regime-aware target portfolio.
- Executes only through Trust Wallet Agent Kit.
- Logs every decision, risk check, quote, transaction, and result.
- Registers the agent on-chain for Track 1.
- Publishes a dashboard and daily signed reports.

## Architecture

Rust core.
Python analytics.
TypeScript dashboard.
TWAK execution.
CMC data.
BNB SDK identity.

## Quickstart

## Demo

## Submission proof
```

---

## `.env.example`

Never commit real secrets.

```env
# Runtime
APP_ENV=paper
RUST_LOG=info
DATABASE_URL=sqlite://data/guardrail_alpha.db

# CMC
CMC_API_KEY=
CMC_X402_ENABLED=true
CMC_MCP_URL=

# TWAK
TWAK_MODE=local
TWAK_WALLET_NAME=guardrail-alpha
TWAK_MCP_URL=http://127.0.0.1:3000
TWAK_REST_URL=http://127.0.0.1:3001

# BSC
BSC_RPC_URL=
BSC_CHAIN_ID=56
COMPETITION_CONTRACT=0x212c61b9b72c95d95bf29cf032f5e5635629aed5

# Agent
AGENT_NAME=Guardrail Alpha
AGENT_METADATA_URL=
AGENT_POLICY_HASH=

# Dashboard
NEXT_PUBLIC_API_URL=http://localhost:8080
```

---

## `Makefile`

```makefile
setup:
	cargo build
	cd dashboard && pnpm install
	cd python-lab && pip install -r requirements.txt

test:
	cargo test --workspace
	cd dashboard && pnpm test || true

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

paper:
	cargo run -p guardrail-agent -- --config configs/paper.toml

live:
	cargo run -p guardrail-agent -- --config configs/production.toml

backtest:
	cargo run -p guardrail-cli -- backtest --config configs/backtest.toml

dashboard:
	cd dashboard && pnpm dev

register:
	./scripts/register_agent.sh

kill:
	./scripts/kill_switch.sh
```

---

# Rust workspace

Top-level `Cargo.toml`:

```toml
[workspace]
resolver = "2"

members = [
  "crates/common",
  "crates/market-data",
  "crates/cmc-client",
  "crates/feature-engine",
  "crates/strategy-engine",
  "crates/risk-engine",
  "crates/portfolio",
  "crates/execution",
  "crates/twak-client",
  "crates/bnb-agent",
  "crates/event-store",
  "crates/backtester",
  "crates/policy-compiler",
  "crates/llm-interface",
  "crates/observability",
  "crates/agent-runtime",
  "apps/guardrail-agent",
  "apps/guardrail-api",
  "apps/guardrail-cli",
  "apps/guardrail-monitor"
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1", features = ["serde"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono", "json"] }
clap = { version = "4", features = ["derive"] }
config = "0.14"
uuid = { version = "1", features = ["serde", "v4"] }
```

---

# `crates/common/`

Shared types. No business logic.

```text
crates/common/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs
    ├── time.rs
    ├── decimal.rs
    ├── chain.rs
    ├── asset.rs
    ├── money.rs
    ├── ids.rs
    ├── config.rs
    └── constants.rs
```

## Important structs

```rust
pub struct Asset {
    pub symbol: String,
    pub cmc_id: u64,
    pub chain_id: u64,
    pub contract_address: String,
    pub decimals: u8,
    pub category: AssetCategory,
}

pub enum AssetCategory {
    Stable,
    Core,
    DeFi,
    Meme,
    AI,
    RWA,
    Infrastructure,
    Other,
}

pub struct Money {
    pub amount: Decimal,
    pub currency: String,
}
```

---

# `crates/cmc-client/`

Only handles CMC communication.

```text
crates/cmc-client/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── client.rs
    ├── rest.rs
    ├── mcp.rs
    ├── x402.rs
    ├── endpoints.rs
    ├── models.rs
    ├── rate_limit.rs
    ├── retry.rs
    └── error.rs
```

## Responsibilities

```text
Fetch latest quotes
Fetch OHLCV
Fetch Fear & Greed
Fetch trending tokens
Fetch DEX pairs
Fetch DEX liquidity
Fetch token security/risk data
Pay for x402 requests when needed
Normalize CMC API responses
Retry and rate-limit safely
```

## Main interface

```rust
#[async_trait]
pub trait CmcDataSource {
    async fn latest_quotes(&self, assets: &[Asset]) -> Result<Vec<CmcQuote>>;
    async fn ohlcv(&self, asset: &Asset, interval: Interval) -> Result<Vec<Candle>>;
    async fn fear_greed(&self) -> Result<FearGreedSnapshot>;
    async fn dex_liquidity(&self, asset: &Asset) -> Result<DexLiquidity>;
    async fn token_security(&self, asset: &Asset) -> Result<TokenSecurity>;
    async fn trending(&self) -> Result<Vec<TrendingToken>>;
}
```

---

# `crates/market-data/`

Normalizes raw CMC/TWAK/external data into internal snapshots.

```text
crates/market-data/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── snapshot.rs
    ├── candle.rs
    ├── universe.rs
    ├── liquidity.rs
    ├── security.rs
    ├── market_regime_inputs.rs
    ├── cache.rs
    └── validator.rs
```

## Main structs

```rust
pub struct MarketSnapshot {
    pub timestamp_ms: i64,
    pub assets: Vec<AssetMarketState>,
    pub fear_greed: Option<FearGreedSnapshot>,
    pub global_market: Option<GlobalMarketState>,
}

pub struct AssetMarketState {
    pub asset: Asset,
    pub price_usd: Decimal,
    pub volume_24h_usd: Decimal,
    pub market_cap_usd: Option<Decimal>,
    pub liquidity_usd: Option<Decimal>,
    pub ret_15m: Option<Decimal>,
    pub ret_1h: Option<Decimal>,
    pub ret_4h: Option<Decimal>,
    pub ret_24h: Option<Decimal>,
    pub volatility_1h: Option<Decimal>,
    pub security_flags: Vec<String>,
}
```

This crate is important because the strategy engine should never consume raw CMC JSON.

---

# `crates/feature-engine/`

Computes signals.

```text
crates/feature-engine/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── momentum.rs
    ├── volume.rs
    ├── volatility.rs
    ├── liquidity.rs
    ├── sentiment.rs
    ├── execution_quality.rs
    ├── risk_penalty.rs
    ├── normalization.rs
    ├── scoring.rs
    └── tests.rs
```

## Output

```rust
pub struct AssetFeatures {
    pub symbol: String,
    pub momentum_score: f64,
    pub volume_acceleration_score: f64,
    pub volatility_score: f64,
    pub liquidity_score: f64,
    pub sentiment_score: f64,
    pub execution_quality_score: f64,
    pub risk_penalty: f64,
}
```

---

# `crates/strategy-engine/`

Chooses target portfolio. No execution. No signing.

```text
crates/strategy-engine/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── regime.rs
    ├── alpha_score.rs
    ├── allocator.rs
    ├── rebalance.rs
    ├── exits.rs
    ├── daily_trade.rs
    ├── target_portfolio.rs
    ├── strategy_config.rs
    └── explanation.rs
```

## Main logic

```rust
pub enum MarketRegime {
    RiskOn,
    RiskOff,
    Chop,
    Breakout,
}

pub struct StrategyDecision {
    pub timestamp_ms: i64,
    pub regime: MarketRegime,
    pub target_positions: Vec<TargetPosition>,
    pub proposed_orders: Vec<OrderIntent>,
    pub explanation: StrategyExplanation,
}
```

## Strategy flow

```text
MarketSnapshot
  -> Features
  -> Regime classification
  -> Alpha scores
  -> Target weights
  -> Rebalance orders
  -> Explanation
```

---

# `crates/risk-engine/`

This is the law. Every trade must pass here.

```text
crates/risk-engine/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── policy.rs
    ├── checks/
    │   ├── mod.rs
    │   ├── asset_allowlist.rs
    │   ├── position_limit.rs
    │   ├── daily_loss.rs
    │   ├── total_drawdown.rs
    │   ├── slippage.rs
    │   ├── liquidity.rs
    │   ├── security_flags.rs
    │   ├── stable_reserve.rs
    │   ├── trade_frequency.rs
    │   ├── wallet_balance.rs
    │   └── correlation.rs
    ├── sizing.rs
    ├── throttle.rs
    ├── kill_switch.rs
    ├── approval.rs
    └── audit.rs
```

## Main types

```rust
pub struct RiskPolicy {
    pub max_total_drawdown_pct: Decimal,
    pub max_daily_drawdown_pct: Decimal,
    pub max_position_pct: Decimal,
    pub max_new_position_pct: Decimal,
    pub min_stable_reserve_pct: Decimal,
    pub max_slippage_pct: Decimal,
    pub kill_switch_drawdown_pct: Decimal,
    pub allowed_assets: Vec<String>,
}

pub enum RiskDecision {
    Approved,
    Rejected { reasons: Vec<String> },
    Clipped { new_amount_usd: Decimal, reasons: Vec<String> },
}
```

## Golden rule

```text
No RiskDecision::Approved
=
No TWAK swap
```

---

# `crates/portfolio/`

Portfolio accounting.

```text
crates/portfolio/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── holding.rs
    ├── portfolio_state.rs
    ├── nav.rs
    ├── pnl.rs
    ├── drawdown.rs
    ├── exposure.rs
    ├── trade_accounting.rs
    └── reconciliation.rs
```

## Responsibilities

```text
Track holdings
Track NAV
Track realized/unrealized PnL
Track drawdown
Track daily PnL
Track exposure
Reconcile TWAK balances with internal ledger
Detect drift
```

---

# `crates/twak-client/`

Thin wrapper around TWAK CLI/MCP/REST.

```text
crates/twak-client/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── cli.rs
    ├── mcp.rs
    ├── rest.rs
    ├── quote.rs
    ├── swap.rs
    ├── wallet.rs
    ├── portfolio.rs
    ├── risk.rs
    ├── approvals.rs
    ├── x402.rs
    ├── competition.rs
    ├── tx_history.rs
    └── error.rs
```

## Responsibilities

```text
Get wallet address
Get balances
Get portfolio
Quote swap
Execute swap
Register competition wallet
Fetch transaction history
Run token risk checks
Pay via x402 if needed
```

## Main interface

```rust
#[async_trait]
pub trait TwakExecutor {
    async fn wallet_address(&self) -> Result<Address>;
    async fn portfolio(&self) -> Result<TwakPortfolio>;
    async fn quote_swap(&self, intent: &OrderIntent) -> Result<SwapQuote>;
    async fn execute_swap(&self, approved: &ApprovedOrder) -> Result<TxReceipt>;
    async fn register_competition(&self) -> Result<TxReceipt>;
}
```

---

# `crates/execution/`

Converts approved orders into TWAK actions.

```text
crates/execution/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── order_intent.rs
    ├── approved_order.rs
    ├── execution_plan.rs
    ├── router.rs
    ├── pre_trade.rs
    ├── post_trade.rs
    ├── retry.rs
    ├── reconciliation.rs
    └── error.rs
```

## Execution flow

```text
OrderIntent
  -> TWAK quote
  -> risk check again
  -> ApprovedOrder
  -> TWAK execute
  -> TxReceipt
  -> portfolio reconciliation
  -> event log
```

---

# `crates/bnb-agent/`

BNB AI Agent SDK integration.

```text
crates/bnb-agent/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── identity.rs
    ├── erc8004.rs
    ├── erc8183.rs
    ├── metadata.rs
    ├── report_hash.rs
    ├── registration.rs
    └── proof.rs
```

## Responsibilities

```text
Register agent identity
Publish metadata
Publish strategy hash
Publish policy hash
Publish run report hash
Create judge-facing proof links
```

This is how we show BNB SDK is not cosmetic.

---

# `crates/event-store/`

Append-only event log.

```text
crates/event-store/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── db.rs
    ├── event.rs
    ├── repository.rs
    ├── migrations.rs
    ├── queries.rs
    ├── projections.rs
    └── export.rs
```

## Event types

```rust
pub enum AgentEvent {
    AgentStarted,
    MarketSnapshotReceived,
    RegimeClassified,
    AssetScored,
    PortfolioTargetComputed,
    OrderProposed,
    RiskApproved,
    RiskRejected,
    RiskClipped,
    TwakQuoteReceived,
    TwakSwapSubmitted,
    TxConfirmed,
    PortfolioReconciled,
    DrawdownThrottleActivated,
    KillSwitchTriggered,
    DailyTradeRequirementSatisfied,
    AgentReportPublished,
}
```

## Why this matters

This gives us demo proof:

```text
Why did it trade?
What data did it use?
What was the quote?
What risk checks passed?
What tx hash was produced?
What changed after execution?
```

---

# `crates/backtester/`

Rust backtester for serious strategy testing.

```text
crates/backtester/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── engine.rs
    ├── historical_data.rs
    ├── simulator.rs
    ├── slippage.rs
    ├── gas.rs
    ├── metrics.rs
    ├── benchmark.rs
    ├── report.rs
    └── walk_forward.rs
```

## Backtest outputs

```text
equity curve
drawdown
daily PnL
turnover
trade count
win rate
profit factor
asset attribution
regime attribution
slippage cost
gas cost
daily trade compliance
```

---

# `crates/policy-compiler/`

Converts natural language strategy into strict JSON policy.

```text
crates/policy-compiler/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── schema.rs
    ├── parser.rs
    ├── validator.rs
    ├── defaults.rs
    ├── policy_hash.rs
    └── compiler.rs
```

## Important rule

The LLM can propose a policy, but Rust validates it.

```text
Natural language
  -> LLM proposal
  -> JSON schema validation
  -> Rust policy validation
  -> policy hash
  -> live runtime
```

---

# `crates/llm-interface/`

LLM is for explanation and policy translation, not trade authority.

```text
crates/llm-interface/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── prompts.rs
    ├── policy_prompt.rs
    ├── explanation_prompt.rs
    ├── report_prompt.rs
    ├── guardrails.rs
    └── client.rs
```

## Allowed LLM actions

```text
Translate strategy text into candidate JSON
Explain trades
Summarize daily report
Generate judge-friendly commentary
```

## Forbidden LLM actions

```text
Direct swap
Override risk engine
Edit live policy without validation
Bypass asset allowlist
Bypass drawdown limits
```

---

# `crates/observability/`

Logs, alerts, metrics.

```text
crates/observability/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── logging.rs
    ├── metrics.rs
    ├── alerts.rs
    ├── health.rs
    └── tracing_setup.rs
```

## Alerts

```text
Drawdown > soft limit
Drawdown > hard limit
TWAK quote failure
CMC data stale
Portfolio reconciliation mismatch
Daily trade not satisfied
Slippage too high
Kill switch triggered
Agent crashed/restarted
```

---

# `crates/agent-runtime/`

The main autonomous loop.

```text
crates/agent-runtime/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── runtime.rs
    ├── scheduler.rs
    ├── state_machine.rs
    ├── trading_loop.rs
    ├── data_loop.rs
    ├── reconciliation_loop.rs
    ├── daily_trade_loop.rs
    ├── report_loop.rs
    ├── shutdown.rs
    └── error.rs
```

## Runtime loops

```text
Data loop:              every 1–5 minutes
Strategy loop:          every 15–60 minutes
Risk monitor:           every minute
Portfolio reconcile:    every 5 minutes
Daily trade monitor:    every hour
Report loop:            daily
Health loop:            every 30 seconds
```

---

# Apps

## `apps/guardrail-agent/`

The actual live trading binary.

```text
apps/guardrail-agent/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── args.rs
    ├── bootstrap.rs
    └── wiring.rs
```

Run:

```bash
cargo run -p guardrail-agent -- --config configs/production.toml
```

---

## `apps/guardrail-api/`

Read-only API for dashboard.

```text
apps/guardrail-api/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── routes/
    │   ├── mod.rs
    │   ├── health.rs
    │   ├── portfolio.rs
    │   ├── trades.rs
    │   ├── signals.rs
    │   ├── risk.rs
    │   ├── events.rs
    │   └── proof.rs
    ├── dto/
    └── server.rs
```

Important: this API must be **read-only**. It should not execute trades.

---

## `apps/guardrail-cli/`

Developer/admin CLI.

```text
apps/guardrail-cli/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── commands/
    │   ├── mod.rs
    │   ├── backtest.rs
    │   ├── score.rs
    │   ├── quote.rs
    │   ├── paper.rs
    │   ├── register.rs
    │   ├── policy.rs
    │   ├── report.rs
    │   └── kill_switch.rs
    └── output.rs
```

Commands:

```bash
guardrail backtest --from 2026-05-01 --to 2026-06-10
guardrail score --config configs/paper.toml
guardrail quote --from USDT --to CAKE --amount 10
guardrail register
guardrail policy hash configs/risk_policy.production.json
guardrail kill-switch
```

---

## `apps/guardrail-monitor/`

Small binary for watchdog/health.

```text
apps/guardrail-monitor/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── watchdog.rs
    ├── alerts.rs
    └── checks.rs
```

---

# Config files

## `configs/production.toml`

```toml
[app]
name = "guardrail-alpha"
mode = "live"
database_url = "sqlite://data/guardrail_alpha.db"

[chain]
chain_id = 56
name = "bsc"

[cmc]
use_rest = true
use_mcp = true
use_x402 = true
request_timeout_ms = 8000

[twak]
mode = "mcp"
quote_before_swap = true
competition_register_enabled = true

[strategy]
loop_interval_seconds = 900
rebalance_threshold_pct = 3
max_positions = 5
min_score_to_enter = 0.65
min_score_to_hold = 0.50

[risk]
policy_path = "configs/risk_policy.production.json"

[reporting]
daily_report_enabled = true
publish_report_hash = true
```

---

## `configs/risk_policy.production.json`

```json
{
  "max_total_drawdown_pct": 22,
  "max_daily_drawdown_pct": 7,
  "max_position_pct": 18,
  "max_new_position_pct": 12,
  "min_stable_reserve_pct": 10,
  "max_slippage_pct": 0.8,
  "kill_switch_drawdown_pct": 24,
  "allowed_chains": [56],
  "execution_layer": "twak_only",
  "require_quote_before_swap": true,
  "daily_trade_requirement": {
    "enabled": true,
    "min_trades_per_day": 1,
    "max_heartbeat_trade_pct": 2
  },
  "forbidden_actions": [
    "launch_token",
    "borrow_without_policy",
    "custodial_signing",
    "trade_non_eligible_assets",
    "bypass_twak"
  ]
}
```

---

## `configs/eligible_assets.bsc.json`

This file is critical because trades outside the eligible list do not count. 

```json
[
  {
    "symbol": "USDT",
    "cmc_id": 825,
    "chain_id": 56,
    "contract_address": "0x...",
    "decimals": 18,
    "category": "stable",
    "enabled": true,
    "min_liquidity_usd": 1000000,
    "min_volume_24h_usd": 1000000
  },
  {
    "symbol": "CAKE",
    "cmc_id": 7186,
    "chain_id": 56,
    "contract_address": "0x...",
    "decimals": 18,
    "category": "defi",
    "enabled": true,
    "min_liquidity_usd": 500000,
    "min_volume_24h_usd": 500000
  }
]
```

---

# Database migrations

Use SQLite first. Easy demo. Easy local reproducibility.

```text
migrations/
├── 0001_init.sql
├── 0002_market_snapshots.sql
├── 0003_trade_events.sql
├── 0004_risk_events.sql
└── 0005_agent_reports.sql
```

## `0001_init.sql`

```sql
CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    mode TEXT NOT NULL,
    strategy_version TEXT NOT NULL,
    policy_hash TEXT NOT NULL,
    wallet_address TEXT
);

CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL
);
```

## `0003_trade_events.sql`

```sql
CREATE TABLE IF NOT EXISTS trade_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    from_symbol TEXT NOT NULL,
    to_symbol TEXT NOT NULL,
    amount_usd REAL NOT NULL,
    status TEXT NOT NULL,
    quote_json TEXT,
    risk_decision_json TEXT,
    tx_hash TEXT,
    reason TEXT
);
```

## `0004_risk_events.sql`

```sql
CREATE TABLE IF NOT EXISTS risk_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    check_name TEXT NOT NULL,
    status TEXT NOT NULL,
    reason TEXT,
    payload_json TEXT NOT NULL
);
```

---

# Python analytics layer

Python should not trade. It reads data and produces charts/reports.

```text
python-lab/
├── pyproject.toml
├── requirements.txt
├── notebooks/
│   ├── 01_universe_filtering.ipynb
│   ├── 02_signal_research.ipynb
│   ├── 03_backtest_review.ipynb
│   ├── 04_live_pnl_analysis.ipynb
│   ├── 05_trade_attribution.ipynb
│   └── 06_submission_charts.ipynb
│
├── scripts/
│   ├── export_equity_curve.py
│   ├── export_drawdown_chart.py
│   ├── export_signal_heatmap.py
│   ├── export_trade_attribution.py
│   ├── generate_daily_report.py
│   └── generate_submission_report.py
│
├── reports/
│   ├── backtest_report.md
│   ├── daily/
│   └── final_submission/
│
├── data/
│   ├── raw/
│   ├── processed/
│   ├── backtests/
│   └── exports/
│
└── guardrail_lab/
    ├── __init__.py
    ├── db.py
    ├── charts.py
    ├── metrics.py
    ├── reports.py
    ├── attribution.py
    └── loaders.py
```

## `requirements.txt`

```text
pandas
numpy
matplotlib
plotly
duckdb
pyarrow
jupyter
pydantic
sqlalchemy
rich
```

Python jobs:

```text
Analyze backtests
Generate charts
Generate PnL reports
Generate drawdown reports
Generate final judge report
Debug strategy behavior
```

Python does **not**:

```text
Call TWAK swap
Hold private keys
Override risk policy
Run live decision loop
```

---

# Dashboard architecture

```text
dashboard/
├── package.json
├── pnpm-lock.yaml
├── next.config.ts
├── tsconfig.json
├── public/
│   ├── logo.svg
│   └── demo/
│
└── src/
    ├── app/
    │   ├── layout.tsx
    │   ├── page.tsx
    │   ├── portfolio/
    │   │   └── page.tsx
    │   ├── trades/
    │   │   └── page.tsx
    │   ├── signals/
    │   │   └── page.tsx
    │   ├── risk/
    │   │   └── page.tsx
    │   ├── proof/
    │   │   └── page.tsx
    │   └── reports/
    │       └── page.tsx
    │
    ├── components/
    │   ├── Layout.tsx
    │   ├── RegimeBadge.tsx
    │   ├── PortfolioTable.tsx
    │   ├── EquityCurve.tsx
    │   ├── DrawdownChart.tsx
    │   ├── SignalTable.tsx
    │   ├── TradeTimeline.tsx
    │   ├── RiskPanel.tsx
    │   ├── GuardrailStatus.tsx
    │   ├── TxHashLink.tsx
    │   └── ProofCard.tsx
    │
    ├── lib/
    │   ├── api.ts
    │   ├── format.ts
    │   └── types.ts
    │
    └── styles/
        └── globals.css
```

## Dashboard pages

```text
/               live cockpit
/portfolio      holdings, NAV, PnL
/trades         trade log and tx hashes
/signals        asset scores and regime
/risk           drawdown, limits, kill switch
/proof          agent address, registration tx, policy hash, report hashes
/reports        daily and final reports
```

Important: dashboard is **read-only**.

---

# CMC Skill companion

This gives us a Track 2-style artifact too.

```text
skills/
└── cmc-regime-routed-alpha/
    ├── README.md
    ├── skill.yaml
    ├── strategy_spec.yaml
    ├── examples/
    │   ├── risk_on_example.json
    │   ├── risk_off_example.json
    │   └── chop_example.json
    ├── prompts/
    │   ├── system.md
    │   ├── strategy_generation.md
    │   └── backtest_spec.md
    └── tests/
        ├── test_strategy_schema.json
        └── test_outputs.json
```

## `skill.yaml`

```yaml
name: regime-routed-bsc-alpha
version: 1.0.0
description: >
  A CMC Skill that turns BSC market, DEX, liquidity, sentiment,
  and Fear & Greed data into a regime-routed crypto trading strategy.

inputs:
  - cmc_quotes
  - cmc_ohlcv
  - cmc_dex_liquidity
  - cmc_fear_greed
  - cmc_trending
  - eligible_asset_list

outputs:
  - market_regime
  - asset_scores
  - target_portfolio
  - entry_rules
  - exit_rules
  - risk_policy
```

---

# Docs folder

This matters for judges.

```text
docs/
├── ARCHITECTURE.md
├── STRATEGY.md
├── RISK.md
├── EXECUTION.md
├── TWAK_INTEGRATION.md
├── CMC_INTEGRATION.md
├── BNB_AGENT_IDENTITY.md
├── LIVE_RUNBOOK.md
├── BACKTEST_METHODOLOGY.md
├── DEMO_SCRIPT.md
└── SUBMISSION_CHECKLIST.md
```

## `ARCHITECTURE.md`

Include:

```text
System diagram
Data flow
Trade flow
Risk flow
Event flow
Why Rust
Why Python only for analytics
Why TS only for dashboard
```

## `STRATEGY.md`

Include:

```text
Universe
Regime detector
Asset scoring
Portfolio construction
Rebalance rules
Daily trade rule
Exit logic
```

## `RISK.md`

Include:

```text
Max drawdown
Daily loss cap
Position cap
Slippage cap
Liquidity checks
Security checks
Stable reserve
Kill switch
Failure modes
```

## `TWAK_INTEGRATION.md`

Include:

```text
Wallet setup
Quote-only flow
Swap flow
Competition registration
Self-custody model
x402 usage
Tx history
Risk checks
```

## `CMC_INTEGRATION.md`

Include:

```text
REST endpoints used
MCP tools used
DEX data used
Fear & Greed usage
x402 usage
Rate limits
Fallbacks
```

## `DEMO_SCRIPT.md`

The demo script should be literally written out:

```text
1. Show natural-language mandate.
2. Show compiled risk policy.
3. Show CMC market data.
4. Show regime classification.
5. Show asset scores.
6. Show risk approval/rejection.
7. Show TWAK quote.
8. Show TWAK signed tx.
9. Show BSC tx hash.
10. Show dashboard and proof page.
```

---

# Test architecture

```text
tests/
├── fixtures/
│   ├── cmc_quotes_sample.json
│   ├── cmc_ohlcv_sample.json
│   ├── cmc_dex_liquidity_sample.json
│   ├── twak_portfolio_sample.json
│   ├── twak_quote_sample.json
│   ├── risk_policy_sample.json
│   └── eligible_assets_sample.json
│
├── integration/
│   ├── cmc_client_test.rs
│   ├── twak_client_test.rs
│   ├── risk_engine_test.rs
│   ├── strategy_engine_test.rs
│   └── execution_pipeline_test.rs
│
├── replay/
│   ├── replay_market_day.rs
│   ├── replay_drawdown_event.rs
│   └── replay_trade_sequence.rs
│
└── smoke/
    ├── paper_trade_smoke.rs
    ├── quote_only_smoke.rs
    └── dashboard_api_smoke.rs
```

## Tests we need

```text
Asset outside eligible list is rejected
Position above max cap is clipped
Slippage above limit is rejected
Drawdown soft throttle activates
Drawdown hard throttle activates
Kill switch blocks trades
Stable reserve cannot be drained
Daily trade monitor detects missing trade
TWAK quote failure does not execute
CMC stale data blocks trading
Portfolio reconciliation mismatch alerts
```

---

# Scripts

```text
scripts/
├── setup.sh
├── register_agent.sh
├── paper_trade.sh
├── live_trade.sh
├── run_backtest.sh
├── export_report.sh
├── healthcheck.sh
└── kill_switch.sh
```

## `scripts/register_agent.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Registering Guardrail Alpha competition wallet via TWAK..."
twak compete register
```

Track 1 requires on-chain registration before the live trading window opens. 

---

## `scripts/kill_switch.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Triggering local kill switch..."
cargo run -p guardrail-cli -- kill-switch --reason "manual_operator_trigger"
```

---

# Infra

```text
infra/
├── Dockerfile.agent
├── Dockerfile.api
├── Dockerfile.dashboard
├── prometheus/
│   ├── prometheus.yml
│   └── alerts.yml
├── grafana/
│   └── dashboards/
│       ├── agent-health.json
│       ├── trading-risk.json
│       └── pnl.json
└── systemd/
    ├── guardrail-agent.service
    ├── guardrail-api.service
    └── guardrail-monitor.service
```

## `docker-compose.yml`

```yaml
services:
  agent:
    build:
      context: .
      dockerfile: infra/Dockerfile.agent
    env_file:
      - .env
    volumes:
      - ./data:/app/data
      - ./configs:/app/configs
    command: ["guardrail-agent", "--config", "configs/paper.toml"]

  api:
    build:
      context: .
      dockerfile: infra/Dockerfile.api
    env_file:
      - .env
    ports:
      - "8080:8080"
    volumes:
      - ./data:/app/data

  dashboard:
    build:
      context: .
      dockerfile: infra/Dockerfile.dashboard
    env_file:
      - .env
    ports:
      - "3000:3000"
    depends_on:
      - api
```

---

# GitHub Actions

```text
.github/
└── workflows/
    ├── rust-ci.yml
    ├── dashboard-ci.yml
    ├── python-ci.yml
    ├── docker-build.yml
    └── security.yml
```

## `rust-ci.yml`

```yaml
name: Rust CI

on:
  push:
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets -- -D warnings
      - run: cargo test --workspace
```

---

# Runtime data folder

Do not commit this, but create locally.

```text
data/
├── guardrail_alpha.db
├── logs/
│   ├── agent.log
│   ├── trades.log
│   └── risk.log
├── snapshots/
│   ├── market/
│   ├── portfolio/
│   └── reports/
├── backtests/
└── exports/
```

Add to `.gitignore`:

```gitignore
data/
.env
*.db
*.sqlite
target/
node_modules/
__pycache__/
.ipynb_checkpoints/
```

---

# Core data flow

```text
CMC raw data
  ↓
cmc-client
  ↓
market-data normalized snapshot
  ↓
feature-engine
  ↓
strategy-engine
  ↓
risk-engine pre-trade check
  ↓
twak-client quote-only
  ↓
risk-engine final check
  ↓
twak-client execute swap
  ↓
event-store
  ↓
portfolio reconciliation
  ↓
dashboard + python reports
```

---

# Trade decision flow

```text
1. Agent wakes up
2. Pulls latest CMC data
3. Pulls TWAK portfolio
4. Updates NAV and drawdown
5. Classifies market regime
6. Scores eligible assets
7. Builds target portfolio
8. Creates order intents
9. Runs risk checks
10. Requests TWAK quote
11. Runs final risk check using quote
12. Executes via TWAK
13. Stores tx hash
14. Reconciles portfolio
15. Updates dashboard
```

---

# Module dependency rule

Keep dependencies one-directional.

```text
common
  ↑
market-data     cmc-client     twak-client
  ↑                 ↑              ↑
feature-engine      │              │
  ↑                 │              │
strategy-engine     │              │
  ↑                 │              │
risk-engine         │              │
  ↑                 │              │
execution ──────────┴──────────────┘
  ↑
agent-runtime
  ↑
apps
```

Important rules:

```text
strategy-engine cannot call TWAK
risk-engine cannot call CMC directly
dashboard cannot call TWAK
python cannot call TWAK swap
LLM cannot call execution
execution cannot bypass risk-engine
```

---

# Minimum viable production build

For the hackathon, do not try to build everything perfectly. Build these first:

```text
Priority 1:
- crates/common
- crates/cmc-client
- crates/market-data
- crates/feature-engine
- crates/strategy-engine
- crates/risk-engine
- crates/twak-client
- crates/execution
- crates/event-store
- crates/agent-runtime
- apps/guardrail-agent
- apps/guardrail-api
- dashboard

Priority 2:
- crates/backtester
- python-lab
- docs
- tests
- scripts

Priority 3:
- crates/bnb-agent
- skills/cmc-regime-routed-alpha
- observability
- full CI
```

---

# The folder tree we should actually create first

Start with this lean version:

```text
guardrail-alpha/
├── README.md
├── .env.example
├── Cargo.toml
├── Makefile
├── configs/
│   ├── paper.toml
│   ├── production.toml
│   ├── risk_policy.production.json
│   └── eligible_assets.bsc.json
│
├── crates/
│   ├── common/
│   ├── cmc-client/
│   ├── market-data/
│   ├── feature-engine/
│   ├── strategy-engine/
│   ├── risk-engine/
│   ├── portfolio/
│   ├── twak-client/
│   ├── execution/
│   ├── event-store/
│   └── agent-runtime/
│
├── apps/
│   ├── guardrail-agent/
│   ├── guardrail-api/
│   └── guardrail-cli/
│
├── dashboard/
├── python-lab/
├── docs/
├── migrations/
├── tests/
└── scripts/
```

This is enough to look production-grade and still be buildable.

---

# Final architecture sentence for README

Use this:

> Guardrail Alpha is organized as a Rust workspace with isolated crates for market data, feature computation, strategy selection, risk enforcement, TWAK execution, portfolio accounting, and event storage. Python is reserved for research and analytics, while the TypeScript dashboard is read-only. No component except the Rust execution pipeline can reach TWAK, and no order reaches TWAK unless the Rust risk engine approves it first.

That is the architecture we should build.
