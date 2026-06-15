# Guardrail Alpha

> A Rust-native autonomous trading agent for BNB Smart Chain that turns a
> plain-English mandate into a hashed, machine-verifiable risk policy — then
> trades it with a dual risk gate, a kill switch, and self-custody execution
> through the Trust Wallet Agent Kit.

**One-line pitch:** Natural-language mandate in, hashed policy out, regime-routed
alpha governed by a Rust risk engine that is the *only* door to a TWAK-signed
swap — every decision logged, hashed, and replayable.

---

## Track & prizes

- **Track 1 — Autonomous Trading Agents.**
- **Special prizes targeted:**
  - **Best Use of TWAK** — TWAK is the sole execution layer; self-custody is
    enforced by the type system (executing without a risk approval is a compile
    error), with Mock / REST / MCP / CLI transports, x402 payment signing, and
    autonomous competition registration.
  - **Best Use of Agent Hub (CoinMarketCap)** — all seven CMC data methods, an
    MCP client to the CMC AI Agent Hub, x402 pay-and-retry for premium data, and
    a packaged CMC Skill (`skills/cmc-regime-routed-alpha`).
  - **Best Use of BNB AI Agent SDK** — ERC-8004 / ERC-8183 identity records,
    deterministic agent / registration ids, and `policy_hash` + `report_hash`
    proof commitments with BscScan links.

---

## What it does

Guardrail Alpha runs a closed, governed loop end to end:

```
NL mandate
  ─▶ compiled & SHA-256-hashed RiskPolicy   (policy-compiler)
  ─▶ regime-routed strategy                 (feature-engine + strategy-engine)
  ─▶ RISK GATE — pre-trade reject           (risk-engine)
  ─▶ TWAK quote (read-only, no keys)        (twak-client)
  ─▶ RISK GATE — final approval / clip      (mints ApprovedOrder)
  ─▶ TWAK execution (TWAK signs)            (self-custody)
  ─▶ portfolio reconcile
  ─▶ SQLite append-only event log           (event-store)
  ─▶ run report (data/run_report.json)
  ─▶ API / dashboard / exporter / monitor
```

- **NL mandate → policy.** A plain-English mandate compiles into a validated
  `RiskPolicy` and a SHA-256 `policy_hash`. The same hash is embedded in every
  `AgentStarted` event and served at `GET /policy` — the policy binds the runtime.
- **Market intelligence.** Live quotes, OHLCV, Fear & Greed, DEX liquidity, and
  token-security data from CoinMarketCap. Paper mode swaps in a deterministic CMC
  mock so the whole flow runs **fully offline and reproducibly**.
- **Dual risk gate + kill switch.** An order only reaches execution after the
  Rust risk engine approves it **twice** — pre-trade and again after the quote.
  Soft drawdown throttle at 22%, hard kill switch at 24% (both safely under the
  30% disqualification line); the kill switch halts and stays engaged.
- **Self-custody execution.** Execution flows only through TWAK. `execute_swap`
  requires an engine-minted `ApprovedOrder` that has no public constructor, so
  "execute without risk approval" is unrepresentable — a compile error, not a
  runtime check. Keys never leave the wallet.
- **BNB identity proof.** An on-chain agent identity (agent id = SHA-256 of name
  + wallet), ERC-8004 / ERC-8183 records, and proof commitments
  (`policy_hash`, `report_hash`) with BscScan links.
- **x402.** For premium CMC data, the engine builds the payment payload and
  **TWAK signs the authorization** — pay-per-request without surrendering keys.
- **Full audit surface.** Every step is an append-only `AgentEvent` in SQLite; a
  `run_report.json` snapshot feeds a read-only Axum API, a Next.js dashboard, a
  Prometheus exporter, and a staleness/drawdown/kill-switch monitor.

---

## The strategy

Guardrail Alpha is a **regime-routed, risk-guarded alpha** strategy on a curated
BSC-eligible universe (all `chain_id 56`).

- **Regime routing.** The market is classified into a regime from CMC market and
  sentiment inputs (Fear & Greed, breadth, volatility). The regime selects the
  posture — how aggressively to deploy versus hold the stable reserve.
- **Feature blend.** Each eligible asset is scored 0..1 by `feature-engine` from
  a blend of momentum, volume / liquidity quality, and token-safety signals — a
  single comparable alpha score per asset.
- **Allocator.** `strategy-engine` selects the top-scoring assets up to
  `max_positions`, sizes them under the per-position weight cap, and keeps a
  minimum stable (USDT) reserve. It emits **target positions and `OrderIntent`s
  only** — the strategy crate has no path to the executor.
- **Rebalance band.** Trades fire only when drift exceeds the rebalance
  threshold, suppressing churn and slippage.
- **Daily-trade heartbeat.** Track 1 requires at least one trade per day. If a
  cycle would trade nothing, the runtime injects a compliant, capped heartbeat
  intent and emits `DailyTradeRequirementSatisfied`.
- **Drawdown caps.** Soft `max_total_drawdown_pct = 22` throttles to
  reduce-only; hard `kill_switch_drawdown_pct = 24` halts the agent. Both sit
  under the 30% DQ threshold.

Validate it offline: `guardrail-cli backtest`, `guardrail-cli walk-forward`
(across sentiment regimes), `guardrail-cli compare` (preset comparison), and
`guardrail-sim` (sentiment sweep) all run the **same** strategy + risk +
portfolio engines as the live loop.

---

## On-chain proof

Everything that ties the running agent to its commitments is deterministic and
inspectable.

- **Agent wallet:** `0xA9e5C0FfEe0000000000000000000000000A1b2C3`
  (paper default; overridable via `AGENT_WALLET`).
- **Competition contract:** `0x212c61b9b72c95d95bf29cf032f5e5635629aed5`
  (`crates/common/src/constants.rs::COMPETITION_CONTRACT`).
- **Where tx hashes appear:**
  - `guardrail-cli register` prints `wallet_address`, `competition_contract`,
    `tx_hash`, and the `https://bscscan.com/tx/<hash>` link.
  - Confirmed swaps are logged as `AgentEvent`s in SQLite and surfaced at
    `GET /trades` and `guardrail-replay trades`.
  - `GET /proof` returns the agent id, registration tx, latest report, and run
    report; `guardrail-cli identity` prints the same identity + ERC-8004 record.
- **Commitments:** `policy_hash` (in every `AgentStarted` event and `GET /policy`)
  and `report_hash` over the run report bind the strategy and results on-chain.

Inspect the current run's proof fields in one command:
`guardrail-cli submission`.

---

## How to run

**Paper mode (fully offline, deterministic mocks — no keys, no network):**

```bash
# End-to-end demo: doctor -> policy compile -> paper agent run -> replay
# -> exporter -> backtest / walk-forward / sweep -> markets -> identity -> report
./scripts/demo.sh

# Or step by step:
cargo build
GUARDRAIL_CYCLES=3 cargo run -p guardrail-agent -- --config configs/paper.toml
cargo run -p guardrail-cli -- submission        # concise submission summary
cargo run -p guardrail-api                       # read-only API on :8080
(cd dashboard && pnpm install && pnpm dev)       # dashboard on :3000
```

**Live / competition mode (real CMC + TWAK; keys stay in the wallet):**

```bash
# Register the competition wallet through TWAK (self-custody)
./scripts/register_agent.sh         # or: cargo run -p guardrail-cli -- register --transport cli

# Run the live agent against production config
./scripts/live_trade.sh             # cargo run -p guardrail-agent -- --config configs/production.toml
```

---

## Repo structure

```
apps/                 seven binaries (the product surface)
  guardrail-agent       autonomous trading loop
  guardrail-cli         dev/admin CLI (policy, score, quote, backtest, identity,
                        register, kill-switch, report, submission)
  guardrail-api         Axum read-only HTTP API (:8080)
  guardrail-sim         sentiment sweep / walk-forward
  guardrail-replay      read-only audit of the SQLite event log
  guardrail-exporter    Prometheus /metrics sidecar (:9100)
  guardrail-monitor     watchdog: staleness / drawdown / kill switch
  guardrail-doctor      preflight readiness checks
  guardrail-tui         terminal cockpit
crates/               focused library crates; authority flows one way
  common                shared types + constants (competition contract)
  cmc-client            CmcDataSource trait: REST / MCP / x402 / Mock
  market-data           CMC -> MarketSnapshot / RegimeInputs + Universe
  feature-engine        per-asset 0..1 alpha feature scores
  strategy-engine       regime + alpha + allocator (intent only)
  portfolio             NAV / holdings / drawdown / trade accounting
  risk-engine           THE GATE: policy + checks + pre/final approval + kill switch
  twak-client           TwakExecutor trait + mock/competition + x402 signing
  execution             intent -> risk -> quote -> final risk -> execute
  backtester            replays strategy + risk + portfolio offline
  event-store           SQLite append-only AgentEvent log
  policy-compiler       NL mandate -> validated RiskPolicy + SHA-256 hash
  bnb-agent             identity, ERC-8004/8183 records, proof hashes
  llm-interface         advisory-only LLM boundary (never authorizes)
  observability         tracing, metrics, health, alerts
  agent-runtime         wires every crate into the live loop
configs/              paper.toml (offline), production.toml, risk policies, universe
dashboard/            Next.js read-only dashboard
scripts/              demo.sh, register_agent.sh, live_trade.sh, ...
docs/                 ARCHITECTURE, STRATEGY, RISK, SELF_CUSTODY, HACKATHON, ADRs
```

**Trust boundaries:** the LLM is advisory-only, Python is analytics-only, and the
dashboard / API are read-only with no path to TWAK. The risk engine is the sole
gate between intent and a signed swap — enforced by the dependency graph and the
type system, not by convention.

See `docs/HACKATHON.md` for the Track 1 requirement-to-code map and
`docs/SELF_CUSTODY.md` for the self-custody invariant.
