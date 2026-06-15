# Momentum + Volatility-Quality Blend — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it ranks names by **price momentum gated through a
volatility-quality filter**, producing a regime-routed rotation over 20 eligible
BSC tokens, with explicit entry/exit, position-sizing, and risk rules that share
the production Guardrail Rust risk envelope field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## Overview

Momentum works — but not at any volatility. A name that is *barely moving* (dead
vol) has no trend energy: the "trend" stalls and chops. A name that is *moving too
violently* (blow-off vol) is usually parabolic and exhaustion-prone: it
mean-reverts hard and stops you out. The names that actually pay are the ones
trending at a **healthy** realised volatility — enough energy to run, not so much
that the move is a spike.

This Skill therefore **multiplies** a momentum leg by a volatility-quality gate.
It favours the intersection — strong, persistent, risk-adjusted momentum AND a
realised volatility sitting in a target band — and demotes both dead-vol and
blow-off-vol names even when their raw momentum looks strong. Exposure scales UP
where trends persist (breakout, risk_on) and DOWN where they fail (chop,
risk_off).

### Signal

Two legs multiply into a single buy-side tilt in `[0, 1]`:

```
momentum_leg = 0.5 * trend_score + 0.5 * momentum_score
  trend_score    : EMA12 > EMA26 > EMA50, rising, price above the stack
  momentum_score : ATR-normalised positive return + positive MACD histogram
                   (forced to 0 in a downtrend or on a negative MACD histogram)

vol_quality      : bell over realised_vol (annualised %)
  1.0  in the healthy band            (vol_lo 50 .. vol_hi 80)
  ramp on each side                   (floor 40 .. lo 50, hi 80 .. ceiling 110)
  0.0  dead zone (< 40) or blow-off zone (> 110)

blend_tilt  = momentum_leg * vol_quality                 (multiplicative gate)
blend_score = clamp01( base_score adjusted by blend_tilt ) * (1 - security_penalty)
```

The multiply is the whole point: neither leg alone is sufficient. A trendless
name scores ~0; a strongly trending but dead- or blow-off-vol name is haircut.
The security penalty is re-applied as a final haircut so a flagged token can never
be bought into a blend. ATR(14) sets the protective stop.

## When to use

Use this Skill when you want a **quality-filtered trend-capture** tilt on the spot
universe — specifically to:

- Buy strong momentum **only when its volatility is healthy**, avoiding both dead
  names that stall and blow-off names that mean-revert.
- Deploy the most capital in strong, broad advances (breakout) and the least in
  directionless or fearful tapes.
- Let healthy-vol winners run with an ATR stop and a +40% take-profit while
  cutting losers fast at -12%.
- Pair with `mean-reversion-chop` (range-fade) and `volatility-targeted-risk-parity`
  (pure sizing) for regime- and axis-complementary coverage.

## Inputs

`cmc_quotes`, `cmc_ohlcv` (→ EMA stack / MACD / ATR / realised vol via
`crates/indicators`), `realised_vol`, `cmc_fear_greed`, `cmc_dex_liquidity`,
`cmc_token_security`, `cmc_global`, and the `eligible_asset_list`. See
`skill.yaml` for the full list and `strategy_spec.yaml` for field definitions.

## Decision procedure

1. **Classify the regime** (`risk_on` | `risk_off` | `chop` | `breakout`) with
   the top-down breadth + Fear & Greed + median-24h-return rules.
2. **Score** each non-stable asset: `momentum_leg` × `vol_quality` → `blend_tilt`,
   then tilt the base alpha score and apply the security haircut.
3. **Select** entries with `blend_score >= 0.65`, max 5, ordered by score; reduce
   to the cleanest 1-2 in chop and to none (heartbeat only) in risk_off.
4. **Size**: `risk_budget = (100 - 12) * exposure_multiplier`, clamped to
   `[0, 90]`; allocate by score share; cap each name at 17%; surplus → USDT.
5. **Emit** entry / exit / rebalance / trim / heartbeat actions and the target
   book (weights summing to <= 100, USDT reserve remainder).

## Risk guardrails

- Per-name cap 17% (≤ risk policy 18%); minimum stable reserve 10% (target 12%).
- Hard stop-loss 12%; take-profit 40% (let winners run); ATR stop (3.0× ATR).
- Drawdown throttle at 22% (block new buys, exits still allowed); kill switch at
  24% (latching halt).
- Daily-trade requirement: ≥ 1 trade/day; a ≤ 0.10% NAV heartbeat when flat.
- Stables are reserve/quote legs only — never blend candidates.

## Volatility-quality band

| Realised vol (annualised) | `vol_quality` | Interpretation |
|---|---|---|
| `< 40%` (dead) | 0.0 | No trend energy — the move stalls. |
| `40% → 50%` (ramp up) | 0.0 → 1.0 | Energy building toward tradable. |
| `50% → 80%` (healthy) | 1.0 | Tradable trend energy — full credit. |
| `80% → 110%` (ramp down) | 1.0 → 0.0 | Getting hot — start trimming. |
| `> 110%` (blow-off) | 0.0 | Parabolic / exhaustion — haircut to 0. |

## Regime exposure (peaks in breakout)

| Regime | Multiplier | Behaviour |
|---|---|---|
| breakout | 1.10 | Over-deploy — strongest trend persistence. |
| risk_on | 1.00 | Full deployment — momentum pays in an up-tape. |
| chop | 0.40 | Whipsaw risk — only the highest-quality blends, small. |
| risk_off | 0.15 | Failed-trend risk — minimal exposure, mostly reserve. |

## Validation

Every file in `examples/` validates clean against the repository's example
validator (`python-lab/guardrail_lab/skill.py::validate_example` returns `[]`):

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/momentum-volatility-blend/examples'); print('valid' if ex and all(V(e)==[] for e in ex) else 'INVALID', len(ex))"
```

See `prompts/` for the system, strategy-generation, and backtest prompts, and
`tests/` for the schema/output contract checks.
