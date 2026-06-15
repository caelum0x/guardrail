# Conservative Core

**Thesis.** Hold a small basket of the most liquid majors (BTCB, ETH, WBNB) with
a large stable buffer. Survive every regime first; capture upside second. This is
the lowest-variance way to stay invested in BSC majors without babysitting.

**Recommended preset:** `conservative`
(`min_score_to_enter 0.65`, `min_score_to_hold 0.55`, `max_positions 3`,
`target_stable_reserve_pct 30`).

## Mandate (compileable)

```
Trade BTCB, ETH and WBNB on BSC. Keep max drawdown at 15%, daily loss 4%,
max position 12%, stable reserve 30%, slippage 0.4%, kill switch at 18%,
at least 1 trade per day, no leverage.
```

### What the parser extracts

| Field | Value | Phrase matched |
|---|---|---|
| `max_total_drawdown_pct` | 15 | `max drawdown` |
| `max_daily_drawdown_pct` | 4 | `daily loss` |
| `max_position_pct` | 12 | `max position` |
| `min_stable_reserve_pct` | 30 | `stable reserve` |
| `max_slippage_pct` | 0.4 | `slippage` |
| `kill_switch_drawdown_pct` | 18 | `kill switch` |
| `daily_trade_requirement.min_trades_per_day` | 1 | `trade per day` |
| `allowed_assets` | `[BTCB, ETH, WBNB]` | ticker scan |
| `forbidden_actions` | `[borrow_without_policy]` | `no leverage` |

The conservative preset's 30% target stable reserve and the mandate's 30% floor
agree on purpose: the policy is the hard floor, the preset is the day-to-day
target.

## Compile and run

```bash
cargo run -p guardrail-cli -- policy compile "Trade BTCB, ETH and WBNB on BSC. Keep max drawdown at 15%, daily loss 4%, max position 12%, stable reserve 30%, slippage 0.4%, kill switch at 18%, at least 1 trade per day, no leverage."

cargo run -p guardrail-cli -- backtest --steps 60 --preset conservative
cargo run -p guardrail-sim -- --walk-forward --windows 6 --steps 30 --preset conservative
```

## Expected behavior across regimes

The strategy is routed by the market-regime classifier
(`crates/strategy-engine/src/regime.rs`), which sets an exposure multiplier per
regime:

- **RiskOn** (`x1.0`): fully deploys up to the 12% per-position cap across the
  three majors; stable reserve stays near the 30% floor.
- **Breakout** (`x1.1`): the slight boost is mostly capped by the tight 12%
  position limit, so this profile barely changes — by design, it does not chase.
- **Chop** (`x0.5`): exposure halves; idle capital parks in stables, lifting the
  reserve well above 30%.
- **RiskOff** (`x0.2`): exposure collapses to ~20%; the basket is largely in
  USDT/USDC. The 18% kill switch is a backstop it should rarely touch given the
  15% total-drawdown brake firing first.
