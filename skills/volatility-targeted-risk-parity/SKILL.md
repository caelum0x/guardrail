# Volatility-Targeted Risk Parity — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification on a **new axis**: it turns **per-asset realised
volatility** into a **risk-based sizing** decision over 20 eligible BSC tokens —
an inverse-volatility / risk-parity book scaled to a target portfolio volatility,
with explicit entry/exit, sizing, and risk rules that share the production
Guardrail Rust risk envelope field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## A different axis from the four signal-direction Skills

The four sibling Skills —
[`cmc-regime-routed-alpha`](../cmc-regime-routed-alpha/),
[`funding-rate-carry`](../funding-rate-carry/),
[`mean-reversion-chop`](../mean-reversion-chop/), and
[`trend-breakout-momentum`](../trend-breakout-momentum/) — all decide **what** to
buy (signal direction): a multi-factor alpha blend, a carry tilt, a counter-trend
reversion tilt, a breakout tilt. This Skill decides **how much** of each to hold
(risk-based sizing). It expresses **no directional view**. Instead it:

1. Sizes each holding by the **inverse of its realised volatility** so every name
   contributes roughly **equal risk** to the book (risk parity / equal-risk
   contribution), and
2. **Scales gross exposure** up or down to hit a **target portfolio volatility**,
   automatically **de-risking** when realised vol spikes (typically risk_off).

Because it is orthogonal to the four, it is registered as an **additional
standalone strategy** — not part of the four-skill regime-complementary ensemble
core (`ensemble.json` is unchanged).

### Sizing layer

```
eligible_i            : non-stable, liquid, security-clean, realised_vol_i > 0
inv_vol_i             : 1 / max(realised_vol_i, vol_floor=0.05)
raw_parity_weight_i   : inv_vol_i / Σ_j inv_vol_j         (each contributes ~equal risk)
                        (mirrors crates/portfolio-optimizer::inverse_volatility / risk_parity_lite)
security haircut      : raw_parity_weight_i *= (1 - security_penalty_i), then re-normalise

est_book_vol          : Σ_i raw_parity_weight_i * realised_vol_i
target_vol_scalar     : clamp( target_portfolio_vol(0.45) / est_book_vol, 0.20, 1.00 )   # never > 1.0: no leverage
gross                 : 100 * regime_multiplier * target_vol_scalar
weight_i              : min( gross * raw_parity_weight_i , 17% )                          # cap; surplus -> USDT
USDT reserve          : 100 - Σ weight_i                                                  # remainder
```

Low-volatility, liquid names get the largest sleeves and hit the 17% cap first;
high-volatility names get small sleeves so their risk contribution matches.

## When to use

Use this Skill when you want a **diversified, risk-balanced** book on the spot
universe — specifically to:

- Equalise risk contribution across holdings instead of betting on a few names.
- Automatically **de-risk** as realised volatility rises (target-vol scaling).
- Run a low-turnover, lower-drawdown core that can be **composed** with any of the
  four signal-direction Skills (use their candidates, size by inverse vol).

## Inputs

`cmc_quotes`, `cmc_ohlcv` (→ `realised_vol` + ATR(14) via `crates/indicators`),
`realised_vol`, `atr_14`, `cmc_fear_greed`, `cmc_dex_liquidity`,
`cmc_token_security`, `cmc_global`, and the `eligible_asset_list`. See
`skill.yaml` for the full list and `strategy_spec.yaml` for field definitions.

## Decision procedure

1. **Classify the regime** (`risk_on` | `risk_off` | `chop` | `breakout`).
2. **Build the eligible set** (signal-light): liquid, security-clean, non-stable,
   `realised_vol > 0`.
3. **Inverse-vol weights**: `inv_vol = 1/vol`; normalise to parity weights; apply
   the security haircut; re-normalise.
4. **Target-vol scalar**: estimate book vol, scale gross toward 45% target
   (clamped to `[0.20, 1.00]` — no leverage).
5. **Size**: `gross = 100 * regime_multiplier * target_vol_scalar`; per-name cap
   17%; surplus + un-deployed gross → USDT reserve.
6. **Emit** entry / exit / rebalance / trim / heartbeat actions and the target
   book (weights summing to <= 100, USDT reserve remainder).

## Risk guardrails

- Per-name cap 17% (≤ risk policy 18%); minimum stable reserve 10% (target 15%).
- No leverage: `target_vol_scalar <= 1.0`. Hard stop-loss 15%; ATR(14) backstop at
  3.5x; no take-profit (sizing/risk book — let parity rebalance).
- Drawdown throttle at 22% (block new buys, exits still allowed); kill switch at
  24% (latching halt).
- Daily-trade requirement: ≥ 1 trade/day; a ≤ 0.10% NAV heartbeat when flat.
- Stables are reserve/quote legs only — never risk-parity sleeves.

## Regime exposure (de-risks in risk_off)

The **primary** gross scalar is the target-vol scalar; the regime multiplier is a
**secondary** trim.

| Regime | Multiplier | Behaviour |
|---|---|---|
| breakout | 1.00 | Full deployment of the balanced book (vol usually calm). |
| risk_on | 1.00 | Full deployment. |
| chop | 0.80 | Modest trim — directionless, keep dry powder. |
| risk_off | 0.40 | De-risk hard — vol spikes, the target-vol scalar also collapses. |

## Validation

Every file in `examples/` validates clean against the repository's example
validator (`python-lab/guardrail_lab/skill.py::validate_example` returns `[]`):

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/volatility-targeted-risk-parity/examples'); print(all(V(e)==[] for e in ex))"
```

See `prompts/` for the system, strategy-generation, and backtest prompts, and
`tests/` for the schema/output contract checks.
