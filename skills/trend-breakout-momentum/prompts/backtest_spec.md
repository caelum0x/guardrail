# Backtest Spec — Momentum / Breakout

How to evaluate this Skill against the Guardrail backtester
(`crates/backtester`) and the production strategy/risk engines.

## Objective

Confirm that routing exposure by regime and entering only **confirmed, volume-
backed breakouts** produces trend-capturing returns in trending tapes while
avoiding death-by-whipsaw in ranges — and that the risk envelope is never
breached.

## Setup

- **Universe**: the 20 eligible BSC tokens (`configs/eligible_assets.bsc.json`);
  USDT is the reserve/quote leg.
- **Engines**: production `StrategyEngine` + `RiskEngine` + `PortfolioState`, the
  same path as live. The risk engine is the sole gate to execution.
- **Signal**: breakout tilt over EMA stack / MACD histogram / Donchian-20 with a
  `volume_ratio >= 1.5` confirmation gate and ATR(14) trailing stops.
- **Price path**: the backtester's sentiment-driven synthetic path
  (`step_return_24h_pct`); momentum/breakout is expected to outperform in the
  trending segments and under-trade in the flat segments.

## Parameters (from `strategy_spec.yaml`)

| Parameter | Value |
|---|---|
| `min_score_to_enter` | 0.65 |
| `min_score_to_hold` | 0.50 |
| `max_positions` | 5 |
| `max_position_weight_pct` | 17.0 |
| `target_stable_reserve_pct` | 12.0 |
| `stop_loss_pct` | 12.0 |
| `take_profit_pct` | 40.0 |
| `drawdown_throttle_pct` | 22.0 |
| `kill_switch_pct` | 24.0 |
| exposure multipliers | breakout 1.1 · risk_on 1.0 · chop 0.4 · risk_off 0.15 |

## Metrics to report

- Total return vs the buy-and-hold benchmark; **excess return**.
- Max drawdown, volatility, Calmar ratio.
- Trade count and average holding period (breakout should hold winners longer).
- Win rate and profit factor (momentum: fewer, larger winners; many small losses).
- Regime attribution: P&L earned in breakout/risk_on vs given back in chop/risk_off.

## Acceptance criteria

1. No risk-limit breach in any window (per-name cap, reserve floor, slippage).
2. Kill switch latches correctly if drawdown reaches 24%.
3. Exposure tracks the regime multiplier (highest gross in breakout, lowest in
   risk_off).
4. Daily-trade requirement satisfied every day (heartbeat when flat).
5. Positive excess return in predominantly trending walk-forward windows.

## How to run

```bash
# Compile a breakout mandate, then backtest / walk-forward it
cargo run -p guardrail-cli -- backtest --preset aggressive
cargo run -p guardrail-cli -- walk-forward --preset aggressive
```

Compare against `skills/mean-reversion-chop` (its mirror): the two should earn in
opposite regimes, so an ensemble is smoother than either alone.
