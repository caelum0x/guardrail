# Strategy

`strategy-engine::StrategyEngine::decide` runs the full pipeline for one cycle:
`snapshot -> features -> regime -> alpha scores -> target weights ->
rebalance orders -> explanation`. It has no execution authority and never signs
or calls TWAK.

## Regime classifier

`regime::classify(RegimeInputs)` reads market breadth (% of assets advancing),
median 24h return, and the Fear & Greed value, returning one of four regimes.
Each regime carries an exposure multiplier applied during allocation:

| Regime     | Condition (first match wins)                 | Exposure multiplier |
|------------|-----------------------------------------------|---------------------|
| `Breakout` | breadth ≥ 65% AND median > 2% AND F&G ≥ 60   | 1.1 |
| `RiskOn`   | breadth ≥ 55% AND F&G ≥ 50                    | 1.0 |
| `RiskOff`  | breadth ≤ 40% OR F&G ≤ 30 OR median < -2%    | 0.2 |
| `Chop`     | everything else (directionless)              | 0.5 |

## Feature blend (alpha score)

`feature-engine` turns each non-stable asset into six normalized 0..1 feature
scores plus a 0..1 `risk_penalty`. `alpha_score::compute` blends them with the
configured weights (defaults shown), normalizes by the weight sum, then applies
the security penalty as a multiplicative haircut
(`score = normalized * (1 - risk_penalty)`, clamped to 0..1):

| Feature             | Default weight |
|---------------------|----------------|
| momentum            | 0.30 |
| execution_quality   | 0.20 |
| volume acceleration | 0.15 |
| liquidity           | 0.15 |
| volatility          | 0.10 |
| sentiment (F&G)     | 0.10 |

Scores are returned sorted descending. (Sentiment is shared across assets,
derived from the snapshot's Fear & Greed value.)

## Allocator caps

`allocator::build_targets` turns ranked scores into `TargetPosition` weights:

- Select assets with `score >= min_score_to_enter` (default 0.65), capped at
  `max_positions` (default 5).
- Risk budget = `(100 - target_stable_reserve_pct) * regime_multiplier`, where
  `target_stable_reserve_pct` defaults to 15%.
- Each name gets a score-proportional slice of the risk budget, hard-capped at
  `max_position_weight_pct` (default 17%, kept at or below the risk policy's
  `max_position_pct` so targets are never auto-rejected). Surplus above the cap
  falls back to the stable reserve.
- The remainder is allocated to the stable reserve (`USDT`). If nothing
  qualifies, the target is 100% reserve.

## Rebalance threshold

`rebalance::compute_orders` converts weight drift into `OrderIntent`s, gated by
a no-churn band. For each non-reserve target, if `|target - current| <
rebalance_threshold_pct` (default 3%) it is skipped; otherwise a Buy
(reserve → asset) or Sell (asset → reserve) is emitted sized at the delta times
NAV. Any held asset no longer in the target set is fully exited. All risk-asset
trades route through the `USDT` reserve. `exits::should_exit` independently
forces an exit when conviction falls below `min_score_to_hold` (default 0.50).

## Daily-trade heartbeat

`daily_trade` implements the Track 1 minimum-activity requirement.
`satisfied_today` checks whether a trade was booked in the current UTC day.
When a cycle would otherwise sit idle, `heartbeat_order` builds a tiny compliant
round-trip into the top symbol, sized at `max_heartbeat_trade_pct` of NAV
(default 2%). The heartbeat is a normal order: it still passes the full risk
gate and is never an excuse to bypass any control.

## Research tooling

`backtester` re-runs the exact production `strategy + risk + portfolio` path over
a synthetic market path, so research and live trading share one code path. Three
modes are exposed (via `guardrail-cli`, `guardrail-sim`, and the
`/backtest`, `/sweep`, `/walkforward` API routes / dashboard pages):

| Mode | What it does |
|------|--------------|
| Backtest    | Single run over a synthetic path; reports the metrics below |
| Sweep       | Re-runs the backtest across a range of Fear & Greed inputs to show how the regime router shifts exposure/return/drawdown from fearful to greedy markets |
| Walk-forward| Runs a sequence of windows whose sentiment ramps across regimes, then prints a per-window table plus an aggregate line |

Every run is scored against an **equal-weight buy-and-hold benchmark** over the
non-stable universe; `excess_return_pct` is the strategy's alpha over that
benchmark. `BacktestMetrics::from_curve` reports `total_return_pct`,
`max_drawdown_pct`, `trade_count`, `win_rate_pct`, `profit_factor`,
`volatility_pct` (std-dev of step returns), and `calmar_ratio` (total return /
max drawdown). See [BACKTEST_METHODOLOGY.md](./BACKTEST_METHODOLOGY.md).
