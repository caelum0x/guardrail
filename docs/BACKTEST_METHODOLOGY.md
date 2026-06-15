# Backtest Methodology

`backtester::run_backtest` validates the live trading logic by running it over a
**deterministic synthetic market path**. It deliberately reuses the production
crates — `StrategyEngine`, `RiskEngine`, `PortfolioState` — so a backtest
exercises the same code that trades live. Only the data source (synthetic) and
the fill (simulated with slippage + gas) are substituted.

> This is a **model**, not a historical backtest. There is no market data on
> disk: prices are generated from a fixed function of `(symbol, step, fear_greed)`.
> Results are fully reproducible for a given `BacktestConfig`, but they measure
> how the strategy behaves under a stylized regime, not realized past performance.

## Synthetic price path

`synthetic.rs` generates an evolving, multi-phase path rather than a monotone
line, so the backtest sees real drawdown, rebalancing, and risk behavior.

| Piece | Source | Behavior |
| --- | --- | --- |
| Initial price | `initial_price(symbol)` | Seeds each non-stable asset from a symbol hash, `$1.00 .. $11.00`. Stables pinned at `$1`. |
| Oscillation | `step_return_24h_pct` | Near-zero-mean repeating 8-phase wave (`OSC_BP`, in basis points), scaled by the symbol's strength tier (`lead`, 1..5) and phase-offset per symbol so the book is decorrelated. This is pure volatility. |
| Sentiment drift | `step_return_24h_pct` | Per-step drift derived from `fear_greed`: `(fear_greed - 50) * 5` bp. ≈ `-2.5%/step` at F&G 0, flat at 50, `+2.5%/step` at 100. Fearful markets trend down, greedy markets trend up — letting the regime-routed strategy demonstrate capital preservation in down markets. |

Each step's 24h return is `oscillation + drift` (percent). Prices evolve as
`price * (1 + r/100)`, floored at `$0.0001`.

`build_snapshot(...)` assembles a full `MarketSnapshot` per step: evolved prices,
volume, liquidity, 1h/24h returns (1h is 24h/4), a ~3% volatility band, the
Fear & Greed snapshot, and global market context. The same generator seeds the
CLI `score` command.

## Real strategy and risk reused

Each step (`engine.rs`):

1. Evolve prices by the step's 24h return.
2. Establish the buy-and-hold benchmark basket (first step only).
3. Mark the portfolio (`mark_all`), refresh the `DrawdownTracker`, record NAV.
4. Run the **real** `StrategyEngine::decide` against the snapshot and current
   allocation.
5. For each proposed order, build a `RiskContext` from the live portfolio
   (projected position weight, stable reserve, security flags) and run it
   through the **real** `RiskEngine::pre_trade`. **Rejected** orders are dropped;
   **clipped** orders use the reduced amount; non-positive amounts are skipped.
6. Simulate the fill and append NAV to the equity curve.

`BacktestConfig` defaults: **60 steps**, **$10,000** starting NAV (all in the
stable reserve), **Fear & Greed 60**.

## Simulated fills

Approved orders are filled in-process (no chain). The fill fee combines two
components, then `apply_fill` updates the portfolio:

| Component | Source | Detail |
| --- | --- | --- |
| Slippage | `slippage::estimate_pct(amount, liquidity)` | Price impact `= amount/liquidity * 100`, charged at half (`impact/2`) plus a fixed `0.05%` venue spread. Liquidity comes from the traded leg's snapshot; defaults to `$1,000,000` if absent. |
| Gas | `gas::fixed_gas_usd()` | Flat `$0.35` per swap (BSC is cheap). |

`fee_usd = amount * slippage_pct / 100 + gas`. Larger trades against thinner
pools pay proportionally more impact.

## Benchmark and excess (alpha) return

`benchmark.rs` defines an **equal-weight buy-and-hold** basket as the baseline:

- On the first step, `BuyAndHold::establish` splits the starting capital equally
  across all eligible **non-stable** assets at their current prices and records a
  fixed quantity per symbol. It then simply holds (no rebalancing, no trades).
- `BuyAndHold::value(prices)` marks the basket to market at the final prices.
- `return_pct(start, end)` is the percent change of the basket.

The run reports:

- `benchmark_return_pct` — buy-and-hold return over the same path.
- `excess_return_pct` — `total_return_pct - benchmark_return_pct`, i.e. the
  strategy's **alpha** over passively holding the market. Positive excess in
  down regimes is the capital-preservation signal the synthetic drift is
  designed to surface.

## Metrics

`BacktestMetrics::from_curve(starting_nav, equity_curve, trade_count)` derives:

| Metric | Definition |
| --- | --- |
| `total_return_pct` | End-to-end NAV change: `(final_nav - starting_nav) / starting_nav * 100`. |
| `max_drawdown_pct` | Largest peak-to-trough decline along the curve. Tracks a running peak and the worst `(peak - nav) / peak * 100`. |
| `trade_count` | Number of fills booked during the run. |
| `win_rate_pct` | Share of steps with a step-over-step NAV increase (`delta > 0`). |
| `profit_factor` | Gross gains divided by gross losses across steps. `0` when there are no losses. |

All values are rounded for reporting. `report::markdown` renders a run (steps,
starting/final NAV, benchmark/excess, and all metrics) as a Markdown table for
the docs/dashboard.
