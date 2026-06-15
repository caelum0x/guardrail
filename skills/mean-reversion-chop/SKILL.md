# Mean-Reversion / Range-Fade (CHOP-specialised) — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it **fades extremes back toward the mean** —
buying oversold dips (low RSI / lower-Bollinger-band touches) and trimming
overbought stretches — over 20 eligible BSC tokens, with explicit entry/exit,
position-sizing, and risk rules that share the production Guardrail Rust risk
envelope field-for-field. It is **specialised for the CHOP (range-bound)
regime** and deliberately steps aside in trending markets.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## Overview

Momentum strategies buy strength and ride trends; **mean-reversion does the
opposite** — it assumes that in a range-bound market, prices that stretch far
from their statistical centre tend to snap back. The edge is real but
regime-dependent: fading extremes pays in a **chop** (sideways, directionless)
market and *bleeds* in a trend, where an oversold asset keeps getting more
oversold. So this Skill is engineered to be **most active in chop** and to
**step aside in trending risk_on / breakout**, holding a large stable reserve.

The dislocation signal blends two oscillators computed from CMC OHLCV using the
Guardrail `crates/indicators` implementations (read-only reference):

- **RSI(14), Wilder** (`crates/indicators/src/rsi.rs`) — bounded `[0, 100]`.
  `RSI <= 30` is oversold (buy zone); `RSI >= 70` is overbought (trim/avoid);
  `RSI <= 12` is a breakdown, not a fadeable dip.
- **Bollinger(20, 2σ)** (`crates/indicators/src/bollinger.rs`) — middle band is
  the SMA; upper/lower are `mid ± 2·std` (population std). The **%B** position
  `= (price - lower) / (upper - lower)` locates price in the band: `<= 0` at/below
  the lower band (oversold), `>= 1` at/above the upper band (overbought), `0.5`
  at the mid.
- **ATR(14), Wilder** (`crates/indicators/src/atr.rs`) — scales protective
  stops/targets to each asset's volatility, so a wide-range token gets a wider
  stop than a quiet one.

This Skill converts those into a **reversion tilt** that *adds* to the base
alpha score for oversold-but-not-broken names and forces the score to 0 for
overbought ones, then routes exposure by market regime and builds a
risk-bounded target book with a large USDT reserve.

## When to use

Use this Skill when you want a **counter-trend, range-fade** behaviour on top of
the spot universe — specifically to:

- Buy assets that the market has pushed to an **oversold extreme** (low RSI,
  price at/below the lower Bollinger band) and trim them as they revert.
- Sit on a **large dry-powder reserve** so you can keep buying dips as a range
  oscillates.
- **Step aside in trends**: in breakout / risk_on the exposure multiplier shrinks
  the book sharply, and in risk_off it rotates to reserve — because fading a
  falling knife loses money.

It is a sibling to `skills/cmc-regime-routed-alpha` and
`skills/funding-rate-carry`: same universe, same regime *classifier*, same risk
envelope, same decision-payload shape — the difference is the **mean-reversion
alpha** and the **inverted exposure profile** (most active in chop, least in
trends).

## Inputs

| Input | Use |
|-------|-----|
| `cmc_ohlcv` | hourly candles — the series RSI/Bollinger/ATR are computed over |
| `rsi_14` | 14-period Wilder RSI (`crates/indicators/src/rsi.rs`) — oversold/overbought |
| `bollinger_20_2` | 20-period 2σ bands + %B (`crates/indicators/src/bollinger.rs`) — band position |
| `atr_14` | 14-period Wilder ATR (`crates/indicators/src/atr.rs`) — volatility-scaled stops |
| `cmc_quotes` | price, % change 1h/24h/7d, market cap, 24h volume |
| `cmc_fear_greed` | market-wide sentiment (0..100) |
| `cmc_dex_liquidity` | on-chain depth -> liquidity + execution-quality |
| `cmc_token_security` | safety score + flags -> security penalty |
| `cmc_global` | total market cap, BTC dominance (regime sanity check) |
| `eligible_asset_list` | the 20 BSC tokens in `configs/eligible_assets.bsc.json` |

## Decision procedure

1. **Classify the market regime** — `risk_on` / `risk_off` / `chop` / `breakout`
   — from market breadth, the CMC Fear & Greed index, and the median 24h return
   (the same top-down rules as the sibling Skills / `regime.rs`).
2. **Compute a reversion tilt** per asset from RSI(14) and Bollinger %B: a
   triangular preference that peaks when an asset is oversold-but-not-broken
   (RSI in `(12, 30]` and %B `<= 0.20`), decays toward 0 as price returns to the
   band mid, and is forced to 0 when overbought or broken-down.
3. **Tilt the base alpha score**: `reversion_score = clamp01(base_score * (0.7 + 0.6 * reversion_tilt) * (1 - security_penalty))`.
4. **Route exposure** with the score-proportional allocator using the *inverted*
   exposure multiplier (chop **1.0**, risk_on **0.4**, breakout **0.2**, risk_off
   **0.15**): select the top reversion scorers above `min_score_to_enter`, honour
   the per-name cap, hold the large stable reserve.
5. **Emit a decision payload**: regime + per-asset reversion scores + target
   portfolio + entry/exit/trim/heartbeat actions + the effective risk policy.

## Risk guardrails (the Rust engine is the final authority)

- Per-name cap **17%** (policy max 18%); stable reserve **25% target** (>= 10%
  floor — large by design). Surplus over caps falls back to USDT — never rejected.
- `min_score_to_enter` **0.65**, `min_score_to_hold` **0.50**, `max_positions` **5**.
- Stop-loss **12%** hard floor plus an ATR-scaled soft stop (**2.5× ATR**);
  take-profit **25%**; breakdown exit if RSI falls to **<= 12** after entry.
- Drawdown throttle at **22%** total drawdown (block new buys); kill switch
  latches at **24%** (halt trading).
- Exposure multipliers (**inverted**): chop **1.0**, risk_on **0.4**, breakout
  **0.2**, risk_off **0.15**. In trending and risk_off regimes the book rotates
  to reserve even if names look oversold.
- Daily-trade requirement: >= 1 trade/day (heartbeat <= 0.10% NAV when flat).
- Reversion is a **tilt, not an override**: it can re-rank and re-weight
  candidates, but it can never breach a risk limit or buy a security-flagged
  asset that the penalty has scored out.

## Files

```
mean-reversion-chop/
├── skill.yaml              # Skill manifest (inputs/outputs)
├── strategy_spec.yaml      # ⭐ the complete, backtestable strategy spec
├── SKILL.md                # this file
├── README.md               # quick-start summary
├── prompts/
│   ├── system.md           # role + hard constraints for the strategy LLM
│   ├── strategy_generation.md  # step-by-step regeneration recipe
│   └── backtest_spec.md    # how to produce a defensible backtest
├── examples/               # full signal -> decision payloads (one per regime)
└── tests/                  # required-output schema + smoke fixtures
```

See `examples/` for one full payload per regime: `risk_on_example.json`,
`risk_off_example.json`, `chop_example.json`, `breakout_example.json` — note the
book is heavy and active in `chop` and nearly all-reserve in the trending and
risk_off regimes.
