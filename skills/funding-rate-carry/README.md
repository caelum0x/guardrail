# Funding-Rate / Basis Carry — Strategy Skill

**Track 2 (Strategy Skills) deliverable.** A CMC-data-driven, backtestable
strategy spec that tilts a 20-token BSC book toward favourable perpetual-swap
**funding** (negative-for-longs or only mildly positive) and fades crowded,
richly-positive longs — inside the same regime model and risk envelope as
`skills/cmc-regime-routed-alpha`.

This Skill is a *spec*, not an executor. The Rust risk engine remains the final
authority over every trade. The full narrative lives in [`SKILL.md`](./SKILL.md);
the machine-readable spec is [`strategy_spec.yaml`](./strategy_spec.yaml).

## The carry signal

Guardrail's `apps/guardrail-api/src/funding.rs` publishes a synthetic per-hour
funding-rate proxy per non-stable asset:

```
funding_rate_proxy = clamp( ret_24h_pct / 24 + (volatility_1h - 3.0) * 0.01, -1.0, 1.0 )
```

- **Low / negative** funding -> longs are cheap or paid -> attractive carry, tilt **up**.
- **Richly positive** funding -> crowded, expensive longs -> tilt **down** (fade).
- **Deeply negative** funding -> capitulation/forced unwind -> tilt back toward neutral.

The tilt multiplies the base alpha score, the security penalty is re-applied as a
haircut, and the score-proportional allocator builds the target book.

## What this Skill does

1. **Classifies the market regime** — `risk_on` / `risk_off` / `chop` /
   `breakout` — from breadth, Fear & Greed, and the median 24h return.
2. **Computes a funding tilt** from the per-asset funding proxy (triangular
   preference peaking in the favourable-for-longs band).
3. **Tilts the base alpha score** by the funding factor, re-applies the security
   penalty, and ranks candidates.
4. **Routes exposure** with the regime exposure multiplier, a per-name cap, and a
   USDT stable reserve, then emits entry/exit/heartbeat actions.

## Inputs

`funding_rate_proxy`, `cmc_quotes`, `asset_scores` (base alpha), `cmc_fear_greed`,
`cmc_dex_liquidity`, `cmc_token_security`, `cmc_global`, and the 20-token
`eligible_asset_list` (`configs/eligible_assets.bsc.json`).

## Outputs

`market_regime`, `carry_scores` (with funding component), `target_portfolio`
(weights summing to ~100, USDT reserve), `actions`, and the effective
`risk_policy`. One worked payload per regime in `examples/`.

## Files

```
funding-rate-carry/
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
| Funding tilt factor | `0.7 + 0.6 * funding_tilt` (tilt in 0..1) | this spec |
| Favourable funding band (per hour) | `[-0.05, 0.10]` peak; fade above `+0.30` | this spec / `funding.rs` bounds |
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

The spec maps onto the same Rust pipeline as the sibling Skill, so it can be
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
gates, and the metrics a judge should expect — including a **carry-attribution**
breakdown (how much PnL came from the funding tilt vs the base alpha).

## Track-2 framing

Track 2 rewards a backtestable strategy spec built on CMC data. This Skill
delivers a derivatives-aware variant: every threshold is grounded in the shipping
engine, the funding signal is the one Guardrail already computes in
`funding.rs`, there are worked examples for all four regimes, and the Rust risk
engine guarantees no generated proposal can breach the documented limits.
