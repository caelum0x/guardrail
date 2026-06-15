# Backtest Spec — Momentum + Volatility-Quality Blend

How to evaluate this Skill against the Guardrail backtester
(`crates/backtester`) and the production strategy/risk engines.

## Objective

Confirm that gating momentum by a **volatility-quality band** (favouring healthy
realised vol, avoiding both dead and blow-off vol) improves the quality of
trend-capture returns versus raw momentum — fewer exhaustion-top entries, fewer
dead-name stalls — while never breaching the risk envelope.

## Setup

- **Universe**: the 20 eligible BSC tokens (`configs/eligible_assets.bsc.json`);
  USDT is the reserve/quote leg.
- **Engines**: production `StrategyEngine` + `RiskEngine` + `PortfolioState`, the
  same path as live. The risk engine is the sole gate to execution.
- **Signal**: `blend_tilt = momentum_leg * vol_quality` over EMA stack / MACD
  histogram / ATR(14) and trailing realised volatility, with ATR(14) stops.
- **Price path**: the backtester's sentiment-driven synthetic path
  (`step_return_24h_pct`); the blend is expected to outperform raw momentum in
  segments where naive momentum buys exhaustion tops or dead names.

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
| vol band (annualised) | floor 40 · healthy 50-80 · ceiling 110 |
| exposure multipliers | breakout 1.1 · risk_on 1.0 · chop 0.4 · risk_off 0.15 |

## Metrics to report

- Total return vs the buy-and-hold benchmark; **excess return**.
- Max drawdown, volatility, Calmar ratio.
- Trade count and average holding period.
- Win rate and profit factor.
- **Vol-band attribution**: P&L from healthy-vol entries vs avoided P&L on
  dead-vol / blow-off-vol names the gate rejected.
- Regime attribution: P&L earned in breakout/risk_on vs given back in chop/risk_off.

## Acceptance criteria

1. No risk-limit breach in any window (per-name cap, reserve floor, slippage).
2. Kill switch latches correctly if drawdown reaches 24%.
3. Exposure tracks the regime multiplier (highest gross in breakout, lowest in
   risk_off).
4. The vol-quality gate demotes dead-vol and blow-off-vol names: no entry is taken
   with `realised_vol_annual_pct < 40` or `> 110`.
5. Daily-trade requirement satisfied every day (heartbeat when flat).
6. Positive excess return in predominantly trending walk-forward windows, with a
   lower max drawdown than the unfiltered momentum Skill.

## How to run

```bash
cargo run -p guardrail-cli -- backtest --preset aggressive
cargo run -p guardrail-cli -- walk-forward --preset aggressive
```

Compare against `skills/trend-breakout-momentum` (raw breakout) and
`skills/volatility-targeted-risk-parity` (pure vol-based sizing): this Skill sits
between them — momentum direction filtered by a volatility-quality band.
