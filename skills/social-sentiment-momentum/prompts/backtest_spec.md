# Backtest Spec — Social / Sentiment Momentum

How to evaluate this Skill against the Guardrail backtester
(`crates/backtester`) and the production strategy/risk engines.

## Objective

Confirm that routing exposure by regime and entering only **accelerating,
money-confirmed attention** (rising trending rank + volume surge + positive
social momentum) captures attention-driven rotations while FADING hype without
volume and DE-RISKING at sentiment extremes — and that the risk envelope is never
breached.

## Setup

- **Universe**: the 20 eligible BSC tokens (`configs/eligible_assets.bsc.json`);
  USDT is the reserve/quote leg.
- **Engines**: production `StrategyEngine` + `RiskEngine` + `PortfolioState`, the
  same path as live. The risk engine is the sole gate to execution.
- **Signal**: attention tilt over trending-rank velocity / volume surge / social
  momentum with a `volume_ratio >= 1.5` confirmation gate and a Fear & Greed
  sentiment-extreme de-risk gate.
- **Price path**: the backtester's sentiment-driven synthetic path
  (`step_return_24h_pct`); attention momentum is expected to outperform in
  trending/bid segments and under-trade in flat/fearful segments.

## Parameters (from `strategy_spec.yaml`)

| Parameter | Value |
|---|---|
| `min_score_to_enter` | 0.65 |
| `min_score_to_hold` | 0.50 |
| `max_positions` | 5 |
| `max_position_weight_pct` | 17.0 |
| `target_stable_reserve_pct` | 12.0 |
| `stop_loss_pct` | 12.0 |
| `take_profit_pct` | 22.0 |
| `drawdown_throttle_pct` | 22.0 |
| `kill_switch_pct` | 24.0 |
| `volume_confirm_ratio` | 1.5 |
| sentiment-extreme gate | greed >= 80 / fear <= 20 -> factor 0.6 |
| exposure multipliers | breakout 1.1 · risk_on 1.0 · chop 0.4 · risk_off 0.15 |

## Metrics to report

- Total return vs the buy-and-hold benchmark; **excess return**.
- Max drawdown, volatility, Calmar ratio.
- Trade count and average holding period.
- Win rate and profit factor (attention momentum: fewer confirmed entries, more
  rejected hype).
- Hype-fade efficacy: P&L avoided by rejecting `volume_ratio < 1.0` trending spikes.
- Regime attribution: P&L earned in risk_on/breakout vs given back in chop/risk_off.

## Acceptance criteria

1. No risk-limit breach in any window (per-name cap, reserve floor, slippage).
2. Kill switch latches correctly if drawdown reaches 24%.
3. Exposure tracks the regime multiplier and the sentiment gate (lowest gross at
   sentiment extremes and in risk_off).
4. Daily-trade requirement satisfied every day (heartbeat when flat).
5. Unconfirmed trending spikes (`volume_ratio < 1.0`) are never entered.

## How to run

```bash
# Compile a sentiment-momentum mandate, then backtest / walk-forward it
cargo run -p guardrail-cli -- backtest --preset aggressive
cargo run -p guardrail-cli -- walk-forward --preset aggressive
```

Compare against `skills/trend-breakout-momentum` (price-structure momentum) and
`skills/mean-reversion-chop`: attention momentum should earn on different signals,
so an ensemble is smoother than any single Skill.
