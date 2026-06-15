# Social / Sentiment Momentum — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it turns **social + sentiment attention** into a
regime-routed rotation over 20 eligible BSC tokens, with explicit entry/exit,
position-sizing, and risk rules that share the production Guardrail Rust risk
envelope field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## Overview

Most strategies read the **chart**. This one reads the **crowd**. It asks: is
ATTENTION accelerating, and is that attention CONFIRMED by money? A name climbing
the CoinMarketCap trending / most-visited board is getting attention — but
attention without volume is a trap (a shill spike that mean-reverts). The signal
blends three attention legs and gates the whole book on market-wide sentiment.

### Signal

Three attention legs blend into a single buy-side tilt in `[0, 1]`, then a
sentiment gate haircuts it at extremes:

```
trend_score        : trending-rank VELOCITY — climbing the board (rising attention)
volume_component   : 1.0 if volume_ratio >= 1.5 (attention CONFIRMED by money)
                     0.0 if volume_ratio < 1.0  (HYPE WITHOUT VOLUME — fade it)
social_component   : positive social momentum (mentions/views accelerating)

attention_tilt = 0.4 * trend_score + 0.35 * volume_component + 0.25 * social_component
sentiment_gate_factor = 0.6 if fear_greed >= 80 (blowoff) or <= 20 (capitulation), else 1.0
sentiment_score = clamp01( base_score adjusted by attention_tilt * gate ) * (1 - security_penalty)
```

The tilt re-ranks and re-sizes the base alpha score; the security penalty is
re-applied as a haircut so a flagged token can never be bought into a hype spike.

## When to use

Use this Skill when you want a **crowd-attention** tilt on the spot universe —
specifically to:

- Favour names with **accelerating attention CONFIRMED by money** (rising
  trending rank + a volume surge + positive social momentum).
- **Fade hype without volume** (attention spikes on `volume_ratio < 1.0`).
- **De-risk at sentiment extremes** (extreme-greed = blowoff-top risk;
  extreme-fear = capitulation).
- Pair with the price-structure Skills (`trend-breakout-momentum`,
  `mean-reversion-chop`) for signal-diverse ensemble coverage.

## Inputs

`cmc_quotes`, `cmc_trending` (→ `trending_rank_velocity`), `volume_surge`,
`social_attention`, `cmc_fear_greed`, `cmc_dex_liquidity`, `cmc_token_security`,
`cmc_global`, and the `eligible_asset_list`. See `skill.yaml` for the full list
and `strategy_spec.yaml` for field definitions.

## Decision procedure

1. **Classify the regime** (`risk_on` | `risk_off` | `chop` | `breakout`) with
   the top-down breadth + Fear & Greed + median-24h-return rules.
2. **Compute the sentiment gate** (factor 0.6 at extremes, else 1.0).
3. **Score** each non-stable asset via the attention tilt above.
4. **Select** entries with `sentiment_score >= 0.65`, max 5, ordered by score;
   reduce to the cleanest 1-2 in chop and to none (heartbeat only) in risk_off.
5. **Size**: `risk_budget = (100 - 12) * exposure_multiplier`, clamped to
   `[0, 90]`; allocate by score share; cap each name at 17%; surplus → USDT.
6. **Emit** entry / exit / rebalance / trim / heartbeat actions and the target
   book (weights summing to <= 100, USDT reserve remainder).

## Risk guardrails

- Per-name cap 17% (≤ risk policy 18%); minimum stable reserve 10% (target 12%).
- Hard stop-loss 12%; take-profit 22%.
- Drawdown throttle at 22% (block new buys, exits still allowed); kill switch at
  24% (latching halt).
- Daily-trade requirement: ≥ 1 trade/day; a ≤ 0.10% NAV heartbeat when flat.
- Stables are reserve/quote legs only — never attention candidates.

## Regime exposure

| Regime | Multiplier | Behaviour |
|---|---|---|
| breakout | 1.10 | Over-deploy — broad rotation, attention is real. |
| risk_on | 1.00 | Full deployment — attention momentum pays in an up-tape. |
| chop | 0.40 | Noisy attention — only the cleanest money-confirmed name, small. |
| risk_off | 0.15 | Fear chatter — minimal exposure, mostly reserve. |

A sentiment-extreme gate (factor 0.6 at Fear & Greed >= 80 or <= 20) stacks on
top of the regime multiplier to de-risk blowoff-top and capitulation.

## Validation

Every file in `examples/` validates clean against the repository's example
validator (`python-lab/guardrail_lab/skill.py::validate_example` returns `[]`):

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/social-sentiment-momentum/examples'); print(all(V(e)==[] for e in ex))"
```

See `prompts/` for the system, strategy-generation, and backtest prompts, and
`tests/` for the schema/output contract checks.
