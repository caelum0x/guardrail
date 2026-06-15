# Backtest Specification Prompt

Describe — and produce — a defensible backtest of the Mean-Reversion / Range-Fade
(CHOP-specialised) strategy so a judge can reproduce and trust the reported
numbers.

## Replay data

- **Universe**: the 20 eligible BSC tokens (`configs/eligible_assets.bsc.json`).
- **Reversion signals**: RSI(14, Wilder), Bollinger(20, 2σ) %B, and ATR(14,
  Wilder) computed from the OHLCV series using the `crates/indicators`
  implementations (`rsi.rs`, `bollinger.rs`, `atr.rs`). Replay them alongside the
  market snapshot.
- **Source**: CMC historical quotes + OHLCV (`/v2/cryptocurrency/ohlcv/historical`),
  Fear & Greed history (`/v3/fear-and-greed/historical`), and DEX liquidity
  snapshots. The Rust `backtester` crate can also run over a synthetic market path
  for deterministic regression tests.
- **Granularity**: hourly candles; regime + reversion tilt + rebalance evaluated
  each cycle (RSI/Bollinger/ATR recomputed per cycle from the trailing window).
- **Period**: a minimum 90-day window that **explicitly includes a long
  range-bound (chop) stretch** plus at least one trending (risk_on/breakout) leg
  and one risk_off leg; report start/end timestamps. The chop stretch is where
  this strategy should earn its keep.

## Cost & execution assumptions

- **Fees**: PancakeSwap v3 swap fee (0.25% default tier) per leg. Mean-reversion
  turns over more than momentum (it trims winners), so fees matter — model them.
- **Slippage**: modelled from DEX liquidity depth vs clip size; capped at the
  policy `max_slippage_pct` of 0.8%. TWAP splitting for large clips.
- **Gas**: fixed BSC gas estimate per swap (configurable, e.g. $0.20/tx).
- **Quote-before-swap**: required; reject fills that exceed the slippage cap.

## Risk gates applied during replay

- Per-name cap 17% (policy max 18%), stable reserve floor (>= 10%, target 25% —
  large by design).
- Stop-loss 12% hard floor plus an ATR-scaled soft stop (2.5× ATR), take-profit
  25% per position; breakdown exit if RSI falls to <= 12 after entry.
- Reversion gates: never initiate when overbought (RSI >= 70 or %B >= 0.80) or
  broken-down (RSI <= 12); trim/exit a held name as it reverts to the mid.
- Inverted exposure multiplier: chop 1.0, risk_on 0.4, breakout 0.2,
  risk_off 0.15 — step aside in trends.
- Drawdown throttle at 22% total drawdown (block new buys), kill switch latches
  at 24% (halt trading).
- Daily-trade requirement: >= 1 trade/day (heartbeat <= 0.10% NAV when flat).

## Metrics to report

- Total return, CAGR, max drawdown, Sharpe & Sortino, Calmar.
- Hit rate, average win/loss, turnover, time-in-market vs reserve.
- **Per-regime attribution** (PnL contributed in each regime) — the headline
  result should show **most PnL earned in `chop`** and near-flat performance in
  trending regimes (proof the regime specialisation works).
- **Reversion attribution**: PnL attributable to the reversion tilt vs the base
  alpha (compare against the regime-routed-bsc-alpha Skill on the same window).
- Kill-switch / throttle activations and their dates.
- Number of trades and daily-requirement compliance rate.

## How to run

```bash
# Synthetic-path regression backtest (deterministic)
guardrail-cli backtest --steps 720 --preset default

# Compare presets side by side (incl. the base regime-routed Skill)
guardrail-cli backtest-presets --steps 720

# Full event-driven simulation over real CMC + indicator replay data
guardrail-sim --config configs/strategy_presets.json

# Python research / metrics notebooks
#   python-lab/  (stdlib + notebooks for attribution and charts)
```

## Output

A backtest report (markdown) with the metrics above, the equity curve, a
per-regime attribution table (highlighting the chop contribution), a
reversion-attribution breakdown, and an explicit list of every risk-gate
activation. See `docs/BACKTEST_METHODOLOGY.md` for the canonical methodology.
