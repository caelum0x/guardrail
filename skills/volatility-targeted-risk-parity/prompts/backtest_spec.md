# Backtest Spec — Volatility-Targeted Risk Parity

How to evaluate this Skill against the Guardrail backtester
(`crates/backtester`) and the production strategy/risk engines.

## Objective

Confirm that sizing by **inverse volatility (risk parity)** and **scaling gross to
a target portfolio volatility** produces a smoother equity curve than an
equal-weight or score-concentrated book — lower realised volatility and shallower
drawdowns — while de-risking automatically when vol spikes, and never breaching
the risk envelope.

## Setup

- **Universe**: the 20 eligible BSC tokens (`configs/eligible_assets.bsc.json`);
  USDT is the reserve/quote leg.
- **Engines**: production `StrategyEngine` + `RiskEngine` + `PortfolioState`, the
  same path as live. The risk engine is the sole gate to execution.
- **Sizing**: inverse-volatility weights via
  `crates/portfolio-optimizer::inverse_volatility` / `risk_parity_lite`
  (`AllocationMethod::InverseVolatility` / `RiskParity`), scaled by the
  target-vol scalar and the regime multiplier, with ATR(14) protective backstops.
- **Price path**: the backtester's sentiment-driven synthetic path
  (`step_return_24h_pct`); the risk-parity book is expected to give back less in
  high-vol segments and ride calm segments fully deployed.

## Parameters (from `strategy_spec.yaml`)

| Parameter | Value |
|---|---|
| `sizing_method` | inverse_volatility (risk parity) |
| `target_portfolio_vol` | 0.45 |
| `target_vol_scalar` range | [0.20, 1.00] (no leverage) |
| `vol_floor` | 0.05 |
| `realised_vol_window_hours` | 168 |
| `max_sleeves` | 12 |
| `max_position_weight_pct` | 17.0 |
| `target_stable_reserve_pct` | 15.0 (min 10) |
| `stop_loss_pct` / `atr_stop_multiple` | 15.0 / 3.5x |
| `drawdown_throttle_pct` / `kill_switch_pct` | 22.0 / 24.0 |
| exposure multipliers | breakout 1.0 · risk_on 1.0 · chop 0.8 · risk_off 0.4 |

## Metrics to report

- **Realised portfolio volatility** vs the 45% target (the headline metric for a
  vol-targeting book) and vs an equal-weight benchmark.
- Total return vs buy-and-hold; **risk-adjusted** return (Sharpe / Calmar).
- Max drawdown and volatility — expected LOWER than the signal-direction Skills.
- Per-name **risk contribution** dispersion — should be near-equal (risk parity).
- Gross-exposure path vs realised vol — gross should fall as vol rises (de-risk).
- Turnover and average holding period (risk parity is low-turnover).

## Acceptance criteria

1. No risk-limit breach in any window (per-name cap, reserve floor, slippage).
2. Kill switch latches correctly if drawdown reaches 24%.
3. Realised book volatility tracks the 45% target within a tolerance band; gross
   de-risks when realised vol spikes (risk_off).
4. Per-name risk contributions are approximately equalised (risk parity holds).
5. Daily-trade requirement satisfied every day (heartbeat when flat).
6. Lower realised volatility and max drawdown than an equal-weight book over the
   same windows.

## How to run

```bash
# Compile a risk-parity mandate, then backtest / walk-forward it
cargo run -p guardrail-cli -- backtest --preset balanced
cargo run -p guardrail-cli -- walk-forward --preset balanced
```

This Skill is on a **different axis** from the four signal-direction Skills: it is
the SIZING layer. Compare it as a standalone risk book, and note it can be
composed with any of the four to re-size their candidate sets by inverse vol.
