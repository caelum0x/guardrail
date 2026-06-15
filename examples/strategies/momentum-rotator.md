# Momentum Rotator

**Thesis.** Rotate into whatever is scoring highest across majors and liquid
large-caps (WBNB, ETH, BTCB, CAKE, LINK), letting the regime model widen exposure
in trends and pull it back in chop. A balanced, actively-rebalanced core.

**Recommended preset:** `balanced`
(`min_score_to_enter 0.55`, `min_score_to_hold 0.45`, `max_positions 5`,
`target_stable_reserve_pct 15`).

## Mandate (compileable)

```
Trade WBNB, ETH, BTCB, CAKE and LINK on BSC. Max drawdown 22%, daily loss 7%,
max position 18%, stable reserve 12%, slippage 0.8%, kill switch 26%,
2 trades per day, no leverage.
```

### What the parser extracts

| Field | Value | Phrase matched |
|---|---|---|
| `max_total_drawdown_pct` | 22 | `max drawdown` |
| `max_daily_drawdown_pct` | 7 | `daily loss` |
| `max_position_pct` | 18 | `max position` |
| `min_stable_reserve_pct` | 12 | `stable reserve` |
| `max_slippage_pct` | 0.8 | `slippage` |
| `kill_switch_drawdown_pct` | 26 | `kill switch` |
| `daily_trade_requirement.min_trades_per_day` | 2 | `trades per day` |
| `allowed_assets` | `[WBNB, ETH, BTCB, CAKE, LINK]` | ticker scan |
| `forbidden_actions` | `[borrow_without_policy]` | `no leverage` |

## Compile and run

```bash
cargo run -p guardrail-cli -- policy compile "Trade WBNB, ETH, BTCB, CAKE and LINK on BSC. Max drawdown 22%, daily loss 7%, max position 18%, stable reserve 12%, slippage 0.8%, kill switch 26%, 2 trades per day, no leverage."

cargo run -p guardrail-cli -- backtest --steps 60 --preset balanced
cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30
cargo run -p guardrail-cli -- compare --steps 60 --fear-greed 70
```

## Expected behavior across regimes

Routed by `crates/strategy-engine/src/regime.rs`:

- **RiskOn** (`x1.0`): holds up to 5 names near the 18% cap, reserve near 12%.
  The 2-trades-per-day floor keeps it actively rotating toward the top scorers.
- **Breakout** (`x1.1`): leans in — the highest-scoring 1-2 names get pushed
  toward the 18% cap, reserve drops to its 12% floor. This is the profile's best
  regime.
- **Chop** (`x0.5`): exposure halves; rotation churn slows and reserve climbs as
  fewer assets clear the 0.55 entry score.
- **RiskOff** (`x0.2`): cuts to ~20% deployed. The 22% drawdown brake and 26%
  kill switch frame the downside; expect it to sit mostly in stables until breadth
  and fear/greed recover.
