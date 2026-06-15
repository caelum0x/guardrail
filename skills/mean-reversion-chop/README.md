# Mean-Reversion / Range-Fade (CHOP-specialised) — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
strategy spec that **fades extremes back toward the mean** — buying oversold
dips (low RSI / lower-Bollinger-band touches) and trimming overbought stretches
— over a 20-token BSC universe, **specialised for the CHOP (range-bound)
regime**, inside the same regime model and risk envelope as
`skills/cmc-regime-routed-alpha` and `skills/funding-rate-carry`.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade. The full narrative lives in [`SKILL.md`](./SKILL.md);
the machine-readable spec is [`strategy_spec.yaml`](./strategy_spec.yaml).

## The reversion signal

Two oscillators, computed from CMC OHLCV with Guardrail's `crates/indicators`
(read-only reference), locate dislocation:

- **RSI(14), Wilder** (`crates/indicators/src/rsi.rs`), bounded `[0, 100]`:
  `<= 30` oversold (**buy**), `>= 70` overbought (**trim/avoid**), `<= 12`
  breakdown (**no-initiate**).
- **Bollinger(20, 2σ)** (`crates/indicators/src/bollinger.rs`): %B
  `= (price - lower) / (upper - lower)` — `<= 0.20` at/below the lower band
  (**buy**), `>= 0.80` at/above the upper band (**trim**), `0.5` at the SMA mid.
- **ATR(14), Wilder** (`crates/indicators/src/atr.rs`): scales protective
  stops/targets to each asset's volatility.

```
reversion_tilt = 0.6 * rsi_score + 0.4 * pb_score          # in [0, 1]
reversion_score = clamp01(base_score * (0.7 + 0.6 * reversion_tilt) * (1 - security_penalty))
```

- **Oversold-but-not-broken** -> tilt **up** (prime buy).
- **Overbought** (RSI >= 70 or %B >= 0.80) -> tilt **0** (never initiate a long).
- **Broken down** (RSI <= 12) -> tilt **0** (oversold can stay oversold).

The tilt multiplies the base alpha score, the security penalty is re-applied as a
haircut, and the score-proportional allocator builds the target book with a
large USDT reserve.

## What this Skill does

1. **Classifies the market regime** — `risk_on` / `risk_off` / `chop` /
   `breakout` — from breadth, Fear & Greed, and the median 24h return.
2. **Computes a reversion tilt** from RSI(14) and Bollinger %B (triangular
   preference peaking in the oversold-but-not-broken zone).
3. **Tilts the base alpha score** by the reversion factor, re-applies the
   security penalty, and ranks candidates.
4. **Routes exposure** with an **inverted** regime multiplier (most active in
   chop), a per-name cap, and a large USDT reserve, then emits
   entry/exit/trim/heartbeat actions.

## Inputs

`cmc_ohlcv`, `rsi_14`, `bollinger_20_2`, `atr_14`, `cmc_quotes`,
`cmc_fear_greed`, `cmc_dex_liquidity`, `cmc_token_security`, `cmc_global`, and
the 20-token `eligible_asset_list` (`configs/eligible_assets.bsc.json`).

## Outputs

`market_regime`, `reversion_scores` (with RSI/%B component), `target_portfolio`
(weights summing to <= 100, large USDT reserve), `actions`, and the effective
`risk_policy`. One worked payload per regime in `examples/`.

## Files

```
mean-reversion-chop/
├── skill.yaml              # Skill manifest (inputs/outputs)
├── strategy_spec.yaml      # the complete, backtestable strategy spec
├── SKILL.md                # overview, when-to-use, decision procedure, guardrails
├── README.md               # this file
├── prompts/                # system + generation + backtest prompts
├── examples/               # full signal -> decision payloads (one per regime)
└── tests/                  # required-output schema + smoke fixtures
```

## Key parameters (shared with the Rust risk engine)

| Parameter | Value | Source |
|-----------|-------|--------|
| Reversion tilt factor | `0.7 + 0.6 * reversion_tilt` (tilt in 0..1) | this spec |
| RSI oversold / overbought / breakdown | `30` / `70` / `12` | this spec / `rsi.rs` |
| Bollinger %B buy / trim zone | `<= 0.20` / `>= 0.80` (period 20, k=2) | this spec / `bollinger.rs` |
| `min_score_to_enter` | 0.65 | `strategy_config.rs` |
| `min_score_to_hold` | 0.50 | `strategy_config.rs` |
| `max_positions` | 5 | `strategy_config.rs` |
| Per-name cap | 17% (policy max 18%) | `strategy_config.rs` / `policy.rs` |
| Stable reserve | 25% target (>= 10% floor — large by design) | `strategy_config.rs` / `policy.rs` |
| Stop-loss / ATR stop / Take-profit | 12% / 2.5× ATR / 25% | this spec / `atr.rs` |
| Drawdown throttle | 22% total drawdown | `policy.rs` |
| Kill switch | 24% (latching) | `policy.rs` |
| Exposure multipliers (**inverted**) | chop 1.0, risk_on 0.4, breakout 0.2, risk_off 0.15 | this spec / `regime.rs` |
| Daily-trade requirement | >= 1/day (heartbeat <= 0.10% NAV) | `policy.rs` |

## How to backtest it

The spec maps onto the same Rust pipeline as the sibling Skills, so it can be
replayed directly:

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

See `prompts/backtest_spec.md` for the data sources, cost assumptions, risk
gates, and the metrics a judge should expect — including a **per-regime
attribution** that should show the strategy earning its keep in `chop` and
staying nearly flat in trending regimes.

## Track-2 framing

Track 2 rewards a backtestable strategy spec built on CMC data. This Skill
delivers a counter-trend variant: every threshold is grounded in the shipping
engine and the `crates/indicators` RSI/Bollinger/ATR semantics, there are worked
examples for all four regimes, and the Rust risk engine guarantees no generated
proposal can breach the documented limits.
