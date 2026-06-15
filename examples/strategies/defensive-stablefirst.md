# Defensive Stable-First

**Thesis.** Capital preservation above all. Keep a heavy stable reserve and only
dip into the two safest majors (BTCB, WBNB) with small positions. For treasuries
and risk-off mandates that want presence in the market without exposure to a
drawdown.

**Recommended preset:** `conservative`
(`min_score_to_enter 0.65`, `min_score_to_hold 0.55`, `max_positions 3`,
`target_stable_reserve_pct 30`).

## Mandate (compileable)

```
Trade BTCB and WBNB on BSC. Keep max drawdown at 10%, daily loss 3%,
max position 8%, stable reserve 40%, slippage 0.3%, kill switch at 12%,
at least 1 trade per day, no leverage.
```

### What the parser extracts

| Field | Value | Phrase matched |
|---|---|---|
| `max_total_drawdown_pct` | 10 | `max drawdown` |
| `max_daily_drawdown_pct` | 3 | `daily loss` |
| `max_position_pct` | 8 | `max position` |
| `min_stable_reserve_pct` | 40 | `stable reserve` |
| `max_slippage_pct` | 0.3 | `slippage` |
| `kill_switch_drawdown_pct` | 12 | `kill switch` |
| `daily_trade_requirement.min_trades_per_day` | 1 | `trade per day` |
| `allowed_assets` | `[BTCB, WBNB]` | ticker scan |
| `forbidden_actions` | `[borrow_without_policy]` | `no leverage` |

The mandate's 40% stable floor is stricter than the conservative preset's 30%
target, so the policy floor wins — the agent will hold at least 40% stables.

## Compile and run

```bash
cargo run -p guardrail-cli -- policy compile "Trade BTCB and WBNB on BSC. Keep max drawdown at 10%, daily loss 3%, max position 8%, stable reserve 40%, slippage 0.3%, kill switch at 12%, at least 1 trade per day, no leverage."

cargo run -p guardrail-cli -- backtest --steps 60 --preset conservative
cargo run -p guardrail-sim -- --walk-forward --windows 6 --steps 30 --preset conservative
```

## Expected behavior across regimes

Routed by `crates/strategy-engine/src/regime.rs`:

- **RiskOn** (`x1.0`): even fully deployed it tops out at ~16% in risk (two 8%
  positions) with 40%+ in stables — deliberately under-exposed.
- **Breakout** (`x1.1`): the boost is almost entirely absorbed by the tiny 8%
  position cap; the profile does not meaningfully chase trends.
- **Chop** (`x0.5`): exposure halves to a handful of percent; effectively a
  stable fund with a toe in the water.
- **RiskOff** (`x0.2`): near-total stables. The tight 10% total-drawdown brake
  and 12% kill switch make a deep loss structurally hard to reach.
