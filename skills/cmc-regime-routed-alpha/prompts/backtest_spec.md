# Backtest Specification Prompt

Describe — and produce — a defensible backtest of the Regime-Routed BSC Alpha
strategy so a judge can reproduce and trust the reported numbers.

## Replay data

- **Universe**: the 20 eligible BSC tokens (`configs/eligible_assets.bsc.json`).
- **Source**: CMC historical quotes + OHLCV (`/v2/cryptocurrency/ohlcv/historical`),
  Fear & Greed history (`/v3/fear-and-greed/historical`), and DEX liquidity
  snapshots. The Rust `backtester` crate can also run over a synthetic market path
  for deterministic regression tests.
- **Granularity**: hourly candles; regime + rebalance evaluated each cycle.
- **Period**: a minimum 90-day window covering at least one risk_on, one
  risk_off, and one chop stretch; report start/end timestamps.

## Cost & execution assumptions

- **Fees**: PancakeSwap v3 swap fee (0.25% default tier) per leg.
- **Slippage**: modelled from DEX liquidity depth vs clip size; capped at the
  policy `max_slippage_pct` of 0.8%. TWAP splitting for large clips.
- **Gas**: fixed BSC gas estimate per swap (configurable, e.g. $0.20/tx).
- **Quote-before-swap**: required; reject fills that exceed the slippage cap.

## Risk gates applied during replay

- Per-name cap 17% (policy max 18%), stable reserve floor (>= 10%, target 15%).
- Stop-loss 12%, take-profit 25% per position.
- Drawdown throttle at 22% total drawdown (block new buys), kill switch latches
  at 24% (halt trading).
- Daily-trade requirement: >= 1 trade/day (heartbeat <= 0.10% NAV when flat).

## Metrics to report

- Total return, CAGR, max drawdown, Sharpe & Sortino, Calmar.
- Hit rate, average win/loss, turnover, time-in-market vs reserve.
- Per-regime attribution (PnL contributed in each regime).
- Kill-switch / throttle activations and their dates.
- Number of trades and daily-requirement compliance rate.

## How to run

```bash
# Synthetic-path regression backtest (deterministic)
guardrail-cli backtest --steps 720 --preset default

# Compare presets side by side
guardrail-cli backtest-presets --steps 720

# Full event-driven simulation over real CMC replay data
guardrail-sim --config configs/strategy_presets.json

# Python research / metrics notebooks
#   python-lab/  (stdlib + notebooks for attribution and charts)
```

## Output

A backtest report (markdown) with the metrics above, the equity curve, a
per-regime attribution table, and an explicit list of every risk-gate activation.
See `docs/BACKTEST_METHODOLOGY.md` for the canonical methodology.
