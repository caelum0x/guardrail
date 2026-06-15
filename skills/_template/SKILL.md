# &lt;PLACEHOLDER_SKILL_NAME&gt; — Strategy Skill (TEMPLATE)

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
trading-strategy specification: it routes exposure by market regime over 20
eligible BSC tokens and tilts the book with <PLACEHOLDER this skill's signal>,
with explicit entry/exit, position-sizing, and risk rules that share the
production Guardrail Rust risk envelope field-for-field.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade.

> **Authoring note (delete after customising):** This file is a skeleton produced
> by `scripts/new_skill.sh`. The only section that meaningfully differs between
> skills is the **signal/tilt** (section 4 of `strategy_spec.yaml`). Replace every
> `<PLACEHOLDER>`, describe your distinguishing signal, and keep the four
> `examples/*.json` validator-clean.

## Overview

<PLACEHOLDER thesis> Explain the market inefficiency or edge this skill targets,
the raw signal it derives from CMC (or Guardrail-computed) data, and why the tilt
re-ranks candidates the way it does. State explicitly that the tilt can re-rank
and re-size but never breach a risk limit.

## When to use

Use this Skill when you want <PLACEHOLDER the specific tilt> on top of the shared
spot universe — specifically to:

- <PLACEHOLDER preference 1: which assets it leans toward and why>.
- <PLACEHOLDER preference 2: what it fades / avoids and why>.
- De-risk into reserve when the market turns `risk_off`, regardless of how
  attractive the signal looks (the signal does not override capital preservation).

It is a sibling to the other Track-2 skills: same universe, same regime model,
same risk envelope, same decision-payload shape — the difference is the
**signal/tilt** layered onto the per-asset score.

## Inputs

| Input | Use |
|-------|-----|
| `<PLACEHOLDER signal feed>` | the distinguishing signal for this skill |
| `cmc_quotes` | price, % change 1h/24h/7d, market cap, 24h volume |
| `asset_scores` | base 0..1 alpha score (reused/derived from the shared blend) |
| `cmc_fear_greed` | market-wide sentiment (0..100) |
| `cmc_dex_liquidity` | on-chain depth -> liquidity + execution-quality |
| `cmc_token_security` | safety score + flags -> security penalty |
| `cmc_global` | total market cap, BTC dominance (regime sanity check) |
| `eligible_asset_list` | the 20 BSC tokens in `configs/eligible_assets.bsc.json` |

## Decision procedure

1. **Classify the market regime** — `risk_on` / `risk_off` / `chop` / `breakout`
   — from market breadth, the CMC Fear & Greed index, and the median 24h return
   (the shared top-down rules / `regime.rs`).
2. **Compute a signal tilt** per asset from <PLACEHOLDER raw signal>: <PLACEHOLDER
   one-line description of the tilt's shape, in 0..1>.
3. **Tilt the base alpha score**: `score = clamp01(base_score * (0.7 + 0.6 * tilt))`
   then re-apply the security penalty as a multiplicative haircut.
4. **Route exposure** with the score-proportional allocator: select the top
   scorers above `min_score_to_enter`, scale the risk budget by the regime's
   exposure multiplier, honour the per-name cap, hold the stable reserve.
5. **Emit a decision payload**: regime + per-asset scores + target portfolio +
   entry/exit/heartbeat actions + the effective risk policy.

## Risk guardrails (the Rust engine is the final authority)

- Per-name cap **17%** (policy max 18%); stable reserve **15% target** (>= 10%
  floor). Surplus over caps falls back to USDT — never rejected.
- `min_score_to_enter` **0.65**, `min_score_to_hold` **0.50**, `max_positions` **5**.
- Stop-loss **12%**, take-profit **25%** per position.
- Drawdown throttle at **22%** total drawdown (block new buys); kill switch
  latches at **24%** (halt trading).
- Exposure multipliers: breakout **1.1**, risk_on **1.0**, chop **0.5**,
  risk_off **0.2**. In `risk_off` the book rotates to reserve even if the signal
  is attractive.
- Daily-trade requirement: >= 1 trade/day (heartbeat <= 0.10% NAV when flat).
- The signal is a **tilt, not an override**: it can re-rank and re-weight
  candidates, but it can never breach a risk limit or buy a security-flagged asset
  that the penalty has scored out.

## Files

```
<skill>/
├── skill.yaml              # Skill manifest (inputs/outputs)
├── strategy_spec.yaml      # ⭐ the complete, backtestable strategy spec (customise §4)
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
