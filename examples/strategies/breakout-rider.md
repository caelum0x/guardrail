# Breakout Rider

**Thesis.** Built for trends. Concentrate into a focused set of high-beta liquid
names (WBNB, ETH, CAKE, AVAX, INJ) and let the regime model push exposure when a
broad, well-bid advance is detected. Sits patient in chop, leans hard in breakout.

**Recommended preset:** `balanced`
(`min_score_to_enter 0.55`, `min_score_to_hold 0.45`, `max_positions 5`,
`target_stable_reserve_pct 15`).

## Mandate (compileable)

```
Trade WBNB, ETH, CAKE, AVAX and INJ on BSC. Max drawdown 25%, daily loss 8%,
max position 20%, stable reserve 10%, slippage 0.9%, kill switch 28%,
2 trades per day, no leverage.
```

### What the parser extracts

| Field | Value | Phrase matched |
|---|---|---|
| `max_total_drawdown_pct` | 25 | `max drawdown` |
| `max_daily_drawdown_pct` | 8 | `daily loss` |
| `max_position_pct` | 20 | `max position` |
| `min_stable_reserve_pct` | 10 | `stable reserve` |
| `max_slippage_pct` | 0.9 | `slippage` |
| `kill_switch_drawdown_pct` | 28 | `kill switch` |
| `daily_trade_requirement.min_trades_per_day` | 2 | `trades per day` |
| `allowed_assets` | `[WBNB, ETH, CAKE, AVAX, INJ]` | ticker scan |
| `forbidden_actions` | `[borrow_without_policy]` | `no leverage` |

## Compile and run

```bash
cargo run -p guardrail-cli -- policy compile "Trade WBNB, ETH, CAKE, AVAX and INJ on BSC. Max drawdown 25%, daily loss 8%, max position 20%, stable reserve 10%, slippage 0.9%, kill switch 28%, 2 trades per day, no leverage."

cargo run -p guardrail-cli -- backtest --steps 60 --preset balanced
cargo run -p guardrail-cli -- compare --steps 60 --fear-greed 75
cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30
```

(Use a high `--fear-greed 75` to land the classifier in Breakout/RiskOn, where
this profile is meant to shine.)

## Expected behavior across regimes

Routed by `crates/strategy-engine/src/regime.rs`. The classifier flags
**Breakout** only when breadth >= 65%, median 24h return > 2%, and fear/greed
>= 60 — exactly the conditions this profile is tuned for:

- **Breakout** (`x1.1`): the design target. Top scorers get pushed past their
  base size toward the 20% cap; reserve drops to its 10% floor. Highest exposure
  of any state here.
- **RiskOn** (`x1.0`): fully invested but un-boosted; holds up to 5 names near
  the 20% cap.
- **Chop** (`x0.5`): patience mode — exposure halves and capital parks in stables
  while it waits for breadth to confirm a trend.
- **RiskOff** (`x0.2`): cuts to ~20% deployed. The 25% drawdown brake and 28%
  kill switch are the hard stops for the rare case a "breakout" fails and reverses.
