# Stress Scenario Library

This is the judge-facing "we tested the safety rails" catalog. It documents the
named stress scenarios in [`configs/scenarios/`](../configs/scenarios) and the
exact guardrail response each one is expected to trigger.

Every scenario is a structured config with stable field names that match the
real risk policy ([`configs/risk_policy.production.json`](../configs/risk_policy.production.json),
schema in [`configs/risk_policy.schema.json`](../configs/risk_policy.schema.json))
and the risk engine in [`crates/risk-engine`](../crates/risk-engine). The
machine-readable list lives in
[`configs/scenarios/index.json`](../configs/scenarios/index.json).

Run the offline walkthrough:

```bash
bash scripts/run_scenarios.sh
```

## Risk policy under test

| Field | Production value | What it gates |
|-------|------------------|----------------|
| `max_total_drawdown_pct` | 22 | Total-drawdown check + Hard throttle band |
| `max_daily_drawdown_pct` | 7 | Daily-loss check |
| `max_position_pct` | 18 | Per-position cap |
| `max_new_position_pct` | 12 | Incremental / new-entry cap |
| `min_stable_reserve_pct` | 10 | Stable-reserve floor |
| `max_slippage_pct` | 0.8 | Pre-trade slippage check |
| `kill_switch_drawdown_pct` | 24 | Kill switch hard stop |
| `daily_trade_requirement.max_heartbeat_trade_pct` | 2 | Heartbeat over-trade cap |

## Guardrail responses

The risk engine has four layered protections (deepest last):

- **throttle** — `risk_engine::throttle::drawdown_throttle` moves Normal → Soft → Hard
  as drawdown rises; Soft/Hard block or shrink new entries.
- **reduce-only** — pre-trade checks (slippage, liquidity, position limit, funding
  pressure) cap or reject new risk so the desk only trims/holds.
- **kill switch** — `risk_engine::kill_switch::should_trigger` fires when
  `total_drawdown_pct >= kill_switch_drawdown_pct`; halts all risk, goes flat.
- **stop-loss** — per-position policy stop that exits the offending name.

## Scenario catalog

| # | ID | Label | Stresses | Expected response | Trips at |
|---|----|-------|----------|-------------------|----------|
| 1 | `flash_crash` | Flash Crash | Sharp broad NAV drawdown | **throttle** (Hard) | `total_drawdown_pct` 22.5 ≥ 22, `daily_drawdown_pct` 9 ≥ 7 |
| 2 | `kill_switch_trip` | Kill Switch Trip | Deeper sustained drawdown | **kill switch** | `total_drawdown_pct` 25 ≥ `kill_switch_drawdown_pct` 24 |
| 3 | `funding_spike` | Funding Spike | Rich funding, crowded longs | **reduce-only** | funding proxy +0.95/hr; `max_new_position_pct` cap |
| 4 | `liquidity_crunch` | Liquidity Crunch | Depth collapse / slippage spike | **reduce-only** | `slippage_pct` 2.4 > `max_slippage_pct` 0.8; `liquidity_usd` ≤ 0 |
| 5 | `regime_whipsaw` | Regime Whipsaw | Rapid risk_on↔risk_off flips | **throttle** (Soft) | `total_drawdown_pct` 14 ≥ soft 12; heartbeat cap |

## What each scenario tests

### 1. Flash Crash (`flash_crash.json`) → throttle
A single-session, broad drawdown. High-beta categories (meme −38%, ai −30%, defi
−24%) gap down while stables hold. Total drawdown reaches 22.5% — past
`max_total_drawdown_pct` (22) and into the Hard throttle band — but below the
24% kill-switch line. The total-drawdown and daily-loss checks fire, new entries
are blocked, but the desk is not yet forced flat.

### 2. Kill Switch Trip (`kill_switch_trip.json`) → kill switch
An extended, deeper drawdown driving total drawdown to 25% — at or beyond
`kill_switch_drawdown_pct` (24). `should_trigger` returns true, the kill switch
fires, and the engine goes reduce-only / flat. This is the deepest protection and
supersedes the plain throttle.

### 3. Funding Spike (`funding_spike.json`) → reduce-only
Perpetual funding turns sharply positive (synthetic funding-rate proxy saturating
near its +1.0/hr cap) as the market crowds into leveraged longs. NAV drawdown is
still shallow, so the kill switch/hard throttle stay dormant. Instead the
position-limit check caps incremental risk at `max_new_position_pct` (12) and the
desk refuses to chase crowded longs — effectively reduce-only on funding-rich
names.

### 4. Liquidity Crunch (`liquidity_crunch.json`) → reduce-only
Pool depth collapses on long-tail names; realized swap slippage spikes to 2.4%,
well past `max_slippage_pct` (0.8), and some quotes return zero liquidity. The
pre-trade slippage and liquidity checks reject the swap before it executes
(`require_quote_before_swap` is true). The desk holds / trims rather than taking a
bad fill.

### 5. Regime Whipsaw (`regime_whipsaw.json`) → throttle
The market flips rapidly between risk_on and risk_off across consecutive windows
(fear/greed and breadth swing hard each step). This stresses the regime router and
trade pacing. Drawdown reaches 14% — past the Soft throttle band (12) — while
daily drawdown nears `max_daily_drawdown_pct` (7). The engine throttles exposure
churn, caps rebalance size at `max_new_position_pct`, and the heartbeat cap
(`max_heartbeat_trade_pct` = 2) prevents whipsaw-driven over-trading.

## How the engine responds (traceability)

| Expected response | Code path |
|-------------------|-----------|
| throttle | [`crates/risk-engine/src/throttle.rs`](../crates/risk-engine/src/throttle.rs) `drawdown_throttle`, [`checks/total_drawdown.rs`](../crates/risk-engine/src/checks/total_drawdown.rs), [`checks/daily_loss.rs`](../crates/risk-engine/src/checks/daily_loss.rs) |
| kill switch | [`crates/risk-engine/src/kill_switch.rs`](../crates/risk-engine/src/kill_switch.rs) `should_trigger`, `checks/total_drawdown.rs` |
| reduce-only | [`checks/slippage.rs`](../crates/risk-engine/src/checks/slippage.rs), [`checks/liquidity.rs`](../crates/risk-engine/src/checks/liquidity.rs), [`checks/position_limit.rs`](../crates/risk-engine/src/checks/position_limit.rs) |
| stop-loss | per-position policy stop (strategy/risk exit path) |

The live `GET /scenarios` API endpoint
([`apps/guardrail-api/src/scenarios.rs`](../apps/guardrail-api/src/scenarios.rs))
applies category shocks from `configs/scenarios/market_stress.json` to the latest
run report for a pre-trade desk view; the named library here extends that with
explicit per-protection expectations for stress testing the safety rails.
