# Sentiment Contrarian

**Thesis.** Lean into beaten-down liquid alts (CAKE, UNI, AAVE, LINK, DOT) when
sentiment is washed out, accepting wider swings for mean-reversion upside. The
regime model still governs sizing, so "contrarian" never means "unhedged."

**Recommended preset:** `aggressive`
(`min_score_to_enter 0.50`, `min_score_to_hold 0.40`, `max_positions 8`,
`target_stable_reserve_pct 8`).

## Mandate (compileable)

```
Trade CAKE, UNI, AAVE, LINK and DOT on BSC. Max drawdown 28%, daily loss 9%,
max position 20%, stable reserve 8%, slippage 1.0%, kill switch 32%,
3 trades per day, no leverage.
```

### What the parser extracts

| Field | Value | Phrase matched |
|---|---|---|
| `max_total_drawdown_pct` | 28 | `max drawdown` |
| `max_daily_drawdown_pct` | 9 | `daily loss` |
| `max_position_pct` | 20 | `max position` |
| `min_stable_reserve_pct` | 8 | `stable reserve` |
| `max_slippage_pct` | 1.0 | `slippage` |
| `kill_switch_drawdown_pct` | 32 | `kill switch` |
| `daily_trade_requirement.min_trades_per_day` | 3 | `trades per day` |
| `allowed_assets` | `[CAKE, UNI, AAVE, LINK, DOT]` | ticker scan |
| `forbidden_actions` | `[borrow_without_policy]` | `no leverage` |

## Compile and run

```bash
cargo run -p guardrail-cli -- policy compile "Trade CAKE, UNI, AAVE, LINK and DOT on BSC. Max drawdown 28%, daily loss 9%, max position 20%, stable reserve 8%, slippage 1.0%, kill switch 32%, 3 trades per day, no leverage."

cargo run -p guardrail-cli -- backtest --steps 60 --preset aggressive
cargo run -p guardrail-sim -- --steps 60 --preset aggressive
cargo run -p guardrail-cli -- compare --steps 60 --fear-greed 25
```

(Note the low `--fear-greed 25`: this profile is designed to be evaluated under
fear, where the contrarian thesis lives.)

## Expected behavior across regimes

Routed by `crates/strategy-engine/src/regime.rs`:

- **RiskOff** (`x0.2`): the intended entry window. Sentiment is low, so the
  low 0.50 entry score lets washed-out alts qualify — but exposure is throttled
  to ~20%, so the contrarian bet is sized small while the thesis is unproven.
- **Chop** (`x0.5`): exposure half-on as reversion plays out; the strategy can
  carry several of its 8 allowed slots.
- **RiskOn** (`x1.0`): the reversion has worked; positions ride toward the 20%
  cap with reserve near the 8% floor.
- **Breakout** (`x1.1`): max conviction — the boost plus the wide 20% cap make
  this the strategy's highest-exposure state. The 28% drawdown brake and 32%
  kill switch are the hard stops that keep an aggressive profile guard-railed.
