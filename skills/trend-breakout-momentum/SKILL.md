# Momentum / Breakout — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it turns **price/volume breakout structure** into
a regime-routed momentum rotation over 20 eligible BSC tokens, with explicit
entry/exit, position-sizing, and risk rules that share the production Guardrail
Rust risk envelope field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## Overview

A **breakout** is a close above a meaningful prior high that marks the start (or
continuation) of a trend. But most breakouts that matter share three traits: the
trend is already aligned (a rising EMA stack), momentum is expanding (a positive
MACD histogram), and — critically — the move is **backed by participation**
(volume expanding relative to its trailing average). A breakout *without* volume
is a fakeout that mean-reverts and bleeds the book.

This Skill locates confirmed breakouts and rides them, scaling exposure UP where
trends persist (breakout, risk_on) and CUTTING HARD where they fail (chop,
risk_off). It is the deliberate **mirror image** of
[`mean-reversion-chop`](../mean-reversion-chop/SKILL.md): peak exposure exactly
where reversion steps aside, so the two compose into a smoother ensemble.

### Signal

Three trend/momentum gates plus two confirmation filters blend into a single
buy-side tilt in `[0, 1]`:

```
trend_score        : EMA12 > EMA26 > EMA50, rising and price above the stack
momentum_component : MACD histogram positive and expanding
struct_score       : 1.0 if close > donchian_upper(20) AND volume_ratio >= 1.5
                     0.0 if volume_ratio < 1.0 (unconfirmed breakout / fakeout)

breakout_tilt = 0.4 * trend_score + 0.35 * momentum_component + 0.25 * struct_score
breakout_score = clamp01( base_score adjusted by breakout_tilt ) * (1 - security_penalty)
```

The breakout tilt re-ranks and re-sizes the base alpha score; the security
penalty is re-applied as a haircut so a flagged token can never be bought into a
breakout. ATR(14) sets the trailing stop that rides the trend and the volatility
floor that a real breakout must clear.

## When to use

Use this Skill when you want a **trend-capture** tilt on the spot universe —
specifically to:

- Deploy the most capital in strong, broad, well-bid advances (the breakout
  regime) and the least in directionless or fearful tapes.
- Enter only on **confirmed** breakouts (volume-backed new highs with aligned
  trend) and reject unconfirmed fakeouts.
- Let winners run with an ATR trailing stop and a generous +40% take-profit while
  cutting losers fast at -12%.
- Pair with `mean-reversion-chop` for regime-complementary coverage.

## Inputs

`cmc_quotes`, `cmc_ohlcv` (→ EMA stack / MACD / ATR / Donchian via
`crates/indicators`), `volume_expansion`, `cmc_fear_greed`, `cmc_dex_liquidity`,
`cmc_token_security`, `cmc_global`, and the `eligible_asset_list`. See
`skill.yaml` for the full list and `strategy_spec.yaml` for field definitions.

## Decision procedure

1. **Classify the regime** (`risk_on` | `risk_off` | `chop` | `breakout`) with
   the top-down breadth + Fear & Greed + median-24h-return rules.
2. **Score** each non-stable asset via the breakout tilt above.
3. **Select** entries with `breakout_score >= 0.65`, max 5, ordered by score;
   reduce to the cleanest 1-2 in chop and to none (heartbeat only) in risk_off.
4. **Size**: `risk_budget = (100 - 12) * exposure_multiplier`, clamped to
   `[0, 90]`; allocate by score share; cap each name at 17%; surplus → USDT.
5. **Emit** entry / exit / rebalance / trim / heartbeat actions and the target
   book (weights summing to <= 100, USDT reserve remainder).

## Risk guardrails

- Per-name cap 17% (≤ risk policy 18%); minimum stable reserve 10% (target 12%).
- Hard stop-loss 12%; take-profit 40% (let winners run); ATR trailing stop.
- Drawdown throttle at 22% (block new buys, exits still allowed); kill switch at
  24% (latching halt).
- Daily-trade requirement: ≥ 1 trade/day; a ≤ 0.10% NAV heartbeat when flat.
- Stables are reserve/quote legs only — never breakout candidates.

## Regime exposure (peaks in breakout)

| Regime | Multiplier | Behaviour |
|---|---|---|
| breakout | 1.10 | Over-deploy — strongest trend persistence. |
| risk_on | 1.00 | Full deployment — momentum pays in an up-tape. |
| chop | 0.40 | Whipsaw risk — only the cleanest, volume-backed breakout, small. |
| risk_off | 0.15 | Failed-breakout risk — minimal exposure, mostly reserve. |

## Validation

Every file in `examples/` validates clean against the repository's example
validator (`python-lab/guardrail_lab/skill.py::validate_example` returns `[]`):

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/trend-breakout-momentum/examples'); print(all(V(e)==[] for e in ex))"
```

See `prompts/` for the system, strategy-generation, and backtest prompts, and
`tests/` for the schema/output contract checks.
