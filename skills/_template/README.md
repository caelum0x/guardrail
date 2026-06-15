# &lt;PLACEHOLDER_SKILL_NAME&gt; — Strategy Skill (TEMPLATE)

> **This is the copy-paste skeleton for a new Track-2 Strategy Skill.**
> Create a real skill from it with:
>
> ```bash
> bash scripts/new_skill.sh my-new-strategy-bsc
> ```
>
> Then replace every `<PLACEHOLDER>` and customise section 4 (the signal/tilt) of
> `strategy_spec.yaml`. Validate your examples with `bash scripts/lint_skills.sh`.
> See [`docs/SKILL_AUTHORING.md`](../../docs/SKILL_AUTHORING.md) for the full guide.

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
strategy spec that routes exposure by market regime over the same fixed universe
of 20 eligible BSC tokens and tilts the book with <PLACEHOLDER this skill's
signal>, inside the shared regime model and risk envelope.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade. The full narrative lives in [`SKILL.md`](./SKILL.md);
the machine-readable spec is [`strategy_spec.yaml`](./strategy_spec.yaml).

## What this Skill does

1. **Classifies the market regime** — `risk_on` / `risk_off` / `chop` /
   `breakout` — from breadth, Fear & Greed, and the median 24h return.
2. **Computes a signal tilt** from <PLACEHOLDER raw signal> (in 0..1).
3. **Tilts the base alpha score** by the tilt factor, re-applies the security
   penalty as a haircut, and ranks candidates.
4. **Routes exposure** with the regime exposure multiplier, a per-name cap, and a
   USDT stable reserve, then emits entry/exit/heartbeat actions.

## Inputs

`cmc_quotes`, `asset_scores` (base alpha), `cmc_fear_greed`, `cmc_dex_liquidity`,
`cmc_token_security`, `cmc_global`, the 20-token `eligible_asset_list`
(`configs/eligible_assets.bsc.json`), and <PLACEHOLDER this skill's signal feed>.

## Outputs

`market_regime`, `asset_scores` (with the signal component), `target_portfolio`
(risk weights summing to <=100; remainder is the USDT reserve), `actions`, and the
effective `risk_policy`. One worked payload per regime in `examples/`.

## Files

```
<skill>/
├── skill.yaml              # Skill manifest (inputs/outputs)
├── strategy_spec.yaml      # the complete, backtestable strategy spec (customise §4)
├── SKILL.md                # overview, when-to-use, decision procedure, guardrails
├── README.md               # this file
├── prompts/                # system + generation + backtest prompts
├── examples/               # full signal -> decision payloads (one per regime)
└── tests/                  # required-output schema + smoke fixtures
```

## Key parameters (shared with the Rust risk engine)

| Parameter | Value | Source |
|-----------|-------|--------|
| Tilt factor | `0.7 + 0.6 * tilt` (tilt in 0..1) | this spec |
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

See `prompts/backtest_spec.md` for the data sources, cost assumptions, risk gates,
and the metrics a judge should expect.
