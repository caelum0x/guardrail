# Risk

`risk-engine` is the only gate to execution. `RiskEngine::approve` runs the
pre-trade checks, applies any clip, then re-runs the checks with the live quote
attached. Every check returns a list of human-readable rejection reasons; an
empty list means "pass". No `RiskDecision::Approved` (or `Clipped`) means no
TWAK swap.

## Decision type

`RiskDecision` is one of:

- `Approved` — order proceeds unchanged.
- `Clipped { new_amount_usd, reasons }` — order shrunk to the
  `max_new_position_pct` cap, then proceeds.
- `Rejected { reasons }` — order is dropped.

## Checks (`crates/risk-engine/src/checks`)

Pre-trade (`run_pre_trade_checks`):

| Check | File | Rejects when |
|-------|------|--------------|
| Asset allowlist  | `asset_allowlist.rs` | `from_symbol` or `to_symbol` not in `allowed_assets` |
| Position limit   | `position_limit.rs`  | `target_position_pct > max_position_pct` |
| Daily loss       | `daily_loss.rs`      | `daily_drawdown_pct >= max_daily_drawdown_pct` |
| Total drawdown   | `total_drawdown.rs`  | `total_drawdown_pct >= max_total_drawdown_pct` (and flags kill-switch breach at `kill_switch_drawdown_pct`) |
| Stable reserve   | `stable_reserve.rs`  | `stable_reserve_pct < min_stable_reserve_pct` |
| Security flags   | `security_flags.rs`  | any security flag is present on the asset |

Quote-aware, added in `final_quote_check`:

| Check | File | Rejects when |
|-------|------|--------------|
| Slippage  | `slippage.rs`  | `quote.slippage_pct > max_slippage_pct` |
| Liquidity | `liquidity.rs` | `quote.liquidity_usd <= 0` |

Supporting helpers (used by the runtime/accounting, not in the pre-trade list):
`trade_frequency.rs` (daily-requirement flag), `wallet_balance.rs` (sufficient
balance), `correlation.rs` (placeholder, currently always within limit).

The new-position clip lives in `approval.rs`: if `amount_usd` exceeds
`nav * max_new_position_pct / 100`, the order is clipped to that cap rather than
rejected.

## Policy fields (`RiskPolicy`, defaults)

| Field | Default | Meaning |
|-------|---------|---------|
| `max_total_drawdown_pct`   | 22  | halt trading past this peak-to-trough loss |
| `max_daily_drawdown_pct`   | 7   | daily loss limit |
| `max_position_pct`         | 18  | per-name cap |
| `max_new_position_pct`     | 12  | max single-order size (clip threshold) |
| `min_stable_reserve_pct`   | 10  | reserve floor |
| `max_slippage_pct`         | 0.8 | per-swap slippage cap |
| `kill_switch_drawdown_pct` | 24  | hard kill threshold |
| `allowed_assets`           | `[USDT, CAKE, WBNB]` | asset allowlist (empty = allow all) |
| `allowed_chains`           | `[56]` | BSC only |
| `execution_layer`          | `twak_only` | execution must route through TWAK |
| `require_quote_before_swap`| `true` | quote mandatory before any swap |
| `daily_trade_requirement`  | enabled, `min_trades_per_day=1`, `max_heartbeat_trade_pct=2` | Track 1 activity |
| `forbidden_actions`        | `launch_token`, `borrow_without_policy`, `custodial_signing`, `trade_non_eligible_assets`, `bypass_twak` | hard prohibitions |

## Kill switch (`kill_switch.rs`)

`KillSwitch` is a latching flag with a reason. `should_trigger(drawdown,
threshold)` is true once `drawdown >= kill_switch_drawdown_pct` (default 24%).
The total-drawdown check surfaces this breach as a distinct rejection reason,
and the runtime/CLI can emit a `KillSwitchTriggered` event.

## Throttle (`throttle.rs`)

`drawdown_throttle(drawdown, soft, hard)` returns a `ThrottleState` of `Normal`,
`Soft` (≥ soft threshold), or `Hard` (≥ hard threshold) so the runtime can
reduce or halt activity as drawdown deepens, before the kill switch fires.
