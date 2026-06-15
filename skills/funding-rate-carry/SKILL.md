# Funding-Rate / Basis Carry — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it turns perpetual-swap **funding-rate pressure**
into a regime-routed long/stable basis-carry rotation over 20 eligible BSC tokens,
with explicit entry/exit, position-sizing, and risk rules that share the
production Guardrail Rust risk envelope field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

## Overview

A perpetual-swap whose **funding rate** is high and positive is paying shorts and
charging longs — it signals a crowded, expensive long that is structurally
unattractive to add to. A funding rate that is **negative or only mildly
positive** means longs are cheap (or paid), the classic setup for a long basis
carry: hold spot, collect (or avoid paying) funding, and lean on the assets the
derivatives crowd is *not* piled into.

Guardrail exposes a synthetic per-hour **funding-rate proxy** at
`apps/guardrail-api/src/funding.rs`:

```
funding_rate_proxy = clamp( ret_24h_pct / 24 + (volatility_1h - 3.0) * 0.01, -1.0, 1.0 )
```

i.e. the hourly slice of the 24h return plus a small volatility-deviation term,
clamped to `[-1.0, 1.0]` per hour. Stables are excluded (they have no funding).

This Skill consumes that proxy, converts it into a **carry tilt** that *adds* to
the base alpha score when funding is favourable for longs and *subtracts* when
funding is richly positive (overheated longs), then routes exposure by market
regime and builds a risk-bounded target book with a USDT reserve.

## When to use

Use this Skill when you want a **derivatives-aware** tilt on top of the spot
universe — specifically to:

- Prefer assets the perp crowd is **under-positioned** in (low/negative funding)
  and **fade** crowded, expensive longs (richly-positive funding).
- Harvest a basis-carry style edge without changing the spot-only execution path
  or the risk limits.
- De-risk into reserve when the market turns `risk_off`, regardless of how
  attractive the funding looks (carry does not override capital preservation).

It is a sibling to `skills/cmc-regime-routed-alpha`: same universe, same regime
model, same risk envelope, same decision-payload shape — the difference is the
**funding-carry tilt** layered onto the per-asset score.

## Inputs

| Input | Use |
|-------|-----|
| `funding_rate_proxy` | per-hour funding proxy per asset (`apps/guardrail-api/src/funding.rs`) — the carry signal |
| `cmc_quotes` | price, % change 1h/24h/7d, market cap, 24h volume |
| `asset_scores` | base 0..1 alpha score (reused from the regime-routed Skill) |
| `cmc_fear_greed` | market-wide sentiment (0..100) |
| `cmc_dex_liquidity` | on-chain depth -> liquidity + execution-quality |
| `cmc_token_security` | safety score + flags -> security penalty |
| `cmc_global` | total market cap, BTC dominance (regime sanity check) |
| `eligible_asset_list` | the 20 BSC tokens in `configs/eligible_assets.bsc.json` |

## Decision procedure

1. **Classify the market regime** — `risk_on` / `risk_off` / `chop` / `breakout`
   — from market breadth, the CMC Fear & Greed index, and the median 24h return
   (the same top-down rules as the regime-routed Skill / `regime.rs`).
2. **Compute a funding tilt** per asset from `funding_rate_proxy`: a triangular
   preference that peaks when funding is in the favourable-for-longs band
   (mildly negative to mildly positive) and decays toward 0 as funding becomes
   richly positive (crowded longs) or deeply negative (capitulation).
3. **Tilt the base alpha score**: `carry_score = clamp01(base_score * (0.7 + 0.6 * funding_tilt))`
   then re-apply the security penalty as a multiplicative haircut.
4. **Route exposure** with the same score-proportional allocator: select the top
   carry scorers above `min_score_to_enter`, scale the risk budget by the
   regime's exposure multiplier, honour the per-name cap, hold the stable reserve.
5. **Emit a decision payload**: regime + per-asset carry scores + target
   portfolio + entry/exit/heartbeat actions + the effective risk policy.

## Risk guardrails (the Rust engine is the final authority)

- Per-name cap **17%** (policy max 18%); stable reserve **15% target** (>= 10%
  floor). Surplus over caps falls back to USDT — never rejected.
- `min_score_to_enter` **0.65**, `min_score_to_hold` **0.50**, `max_positions` **5**.
- Stop-loss **12%**, take-profit **25%** per position.
- Drawdown throttle at **22%** total drawdown (block new buys); kill switch
  latches at **24%** (halt trading).
- Exposure multipliers: breakout **1.1**, risk_on **1.0**, chop **0.5**,
  risk_off **0.2**. In `risk_off` the book rotates to reserve even if funding is
  attractive.
- Daily-trade requirement: >= 1 trade/day (heartbeat <= 0.10% NAV when flat).
- Carry is a **tilt, not an override**: it can re-rank and re-weight candidates,
  but it can never breach a risk limit or buy a security-flagged asset that the
  penalty has scored out.

## Files

```
funding-rate-carry/
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
`risk_off_example.json`, `chop_example.json`, `breakout_example.json`.
