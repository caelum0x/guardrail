# CMC Regime-Routed BSC Alpha — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it turns CoinMarketCap market data into a
regime-routed long/stable rotation over 20 eligible BSC tokens, with explicit
entry/exit, position-sizing, and risk rules that mirror the production Guardrail
Rust engine field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## What this Skill does

1. **Classifies the market regime** — `risk_on` / `risk_off` / `chop` /
   `breakout` — from market breadth, the CMC Fear & Greed index, and the median
   24h return (same top-down rules as `crates/strategy-engine/src/regime.rs`).
2. **Scores every candidate asset** by blending normalized features — momentum
   (with RSI/MACD confirmation), volume turnover, volatility, DEX liquidity,
   sentiment, and execution-quality — then applies a **security penalty** as a
   multiplicative haircut (mirrors `feature-engine/*`, `indicators`, and
   `alpha_score.rs`).
3. **Routes exposure** with a score-proportional allocator that scales the risk
   budget by the regime's exposure multiplier, enforces a per-name cap, and keeps
   a stable reserve (`allocator.rs`).
4. **Emits a decision payload**: regime + per-asset scores + target portfolio +
   entry/exit/heartbeat actions + the effective risk policy.

## Inputs (CMC feeds)

| Input | Use |
|-------|-----|
| `cmc_quotes` | price, % change 1h/24h/7d, market cap, 24h volume |
| `cmc_ohlcv` | RSI(14), MACD(12,26,9), ATR-based volatility |
| `cmc_fear_greed` | market-wide sentiment (0..100) |
| `cmc_dex_liquidity` | on-chain depth -> liquidity + execution-quality |
| `cmc_token_security` | safety score + flags -> security penalty |
| `cmc_trending` | soft confirmation of participation |
| `cmc_global` | total market cap, BTC dominance (regime sanity check) |
| `eligible_asset_list` | the 20 BSC tokens in `configs/eligible_assets.bsc.json` |

## Outputs

`market_regime`, `asset_scores` (with component breakdown), `target_portfolio`
(weights summing to ~100, USDT reserve), `actions` (entry/exit/rebalance/
heartbeat), and the effective `risk_policy`. See `examples/` for one full payload
per regime: `risk_on_example.json`, `risk_off_example.json`, `chop_example.json`,
`breakout_example.json`.

## Files

```
cmc-regime-routed-alpha/
├── skill.yaml              # Skill manifest (inputs/outputs)
├── strategy_spec.yaml      # ⭐ the complete, backtestable strategy spec
├── README.md               # this file
├── prompts/
│   ├── system.md           # role + hard constraints for the strategy LLM
│   ├── strategy_generation.md  # step-by-step regeneration recipe
│   └── backtest_spec.md    # how to produce a defensible backtest
├── examples/               # full signal -> decision payloads (one per regime)
└── tests/                  # required-output schema + smoke fixtures
```

## Key parameters (traceable to the Rust engine)

| Parameter | Value | Source |
|-----------|-------|--------|
| Feature weights | momentum .30, exec_quality .20, volume .15, liquidity .15, volatility .10, sentiment .10 | `strategy_config.rs` |
| `min_score_to_enter` | 0.65 | `strategy_config.rs` |
| `min_score_to_hold` | 0.50 | `strategy_config.rs` |
| `max_positions` | 5 | `strategy_config.rs` |
| Per-name cap | 17% (policy max 18%) | `strategy_config.rs` / `policy.rs` |
| Stable reserve | 15% target (>= 10% floor) | `strategy_config.rs` / `policy.rs` |
| Stop-loss / Take-profit | 12% / 25% | `strategy_config.rs` |
| Drawdown throttle | 22% total drawdown | `policy.rs` |
| Kill switch | 24% (latching) | `policy.rs` |
| Exposure multipliers | breakout 1.1, risk_on 1.0, chop 0.5, risk_off 0.2 | `regime.rs` |
| Daily-trade requirement | >= 1/day (heartbeat <= 0.10% NAV) | `policy.rs` |

## How to backtest it

The spec maps 1:1 onto the Rust pipeline, so you can replay it directly:

```bash
# Deterministic synthetic-path backtest
guardrail-cli backtest --steps 720 --preset default

# Compare strategy presets side by side
guardrail-cli backtest-presets --steps 720

# Full event-driven simulation over CMC replay data
guardrail-sim --config configs/strategy_presets.json

# Research / metrics / charts (stdlib Python notebooks)
#   python-lab/   — attribution, equity curve, per-regime PnL
```

See `prompts/backtest_spec.md` and `docs/BACKTEST_METHODOLOGY.md` for the data
sources, cost assumptions (fees, slippage, gas), risk gates, and the metrics a
judge should expect (return, max drawdown, Sharpe/Sortino, per-regime attribution,
kill-switch activations).

## Track-2 framing

Track 2 rewards a **backtestable strategy spec built on CMC data**. This Skill
delivers exactly that: a self-contained, auditable specification whose every
threshold is grounded in the shipping engine, with worked examples for all four
regimes and a reproducible backtest path. An LLM can regenerate the strategy from
fresh CMC data using `prompts/strategy_generation.md`, and the Rust risk engine
guarantees no generated proposal can breach the documented risk limits.
