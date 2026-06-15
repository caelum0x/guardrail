# volatility-targeted-risk-parity

A Track-2 **Strategy Skill** for Guardrail Alpha on a **new axis**: instead of
deciding *what* to buy (signal direction), it decides *how much* of each to hold
(risk-based sizing). It sizes positions by **inverse volatility / risk parity** so
each holding contributes roughly equal risk, then **scales gross exposure to hit a
target portfolio volatility**, de-risking when realised vol spikes. It is the
sizing complement to the four signal-direction Skills
([`cmc-regime-routed-alpha`](../cmc-regime-routed-alpha/),
[`funding-rate-carry`](../funding-rate-carry/),
[`mean-reversion-chop`](../mean-reversion-chop/),
[`trend-breakout-momentum`](../trend-breakout-momentum/)).

> This is a strategy **specification**, not an executor. The Rust risk engine is
> the final authority over every trade. Registered as an **additional standalone
> strategy** — the four-skill regime-complementary ensemble core
> (`../ensemble.json`) is unchanged.

## Files

| File | Purpose |
|---|---|
| `skill.yaml` | Skill manifest — name, inputs, outputs, regimes, example/prompt index. |
| `strategy_spec.yaml` | Machine-readable strategy spec — universe, regime model, inverse-vol/risk-parity sizing, target-vol scaling, risk policy. |
| `SKILL.md` | Human-readable overview, decision procedure, and risk guardrails. |
| `prompts/system.md` | System prompt for the strategy-reasoning layer. |
| `prompts/strategy_generation.md` | Single-cycle decision-generation prompt. |
| `prompts/backtest_spec.md` | How to backtest / walk-forward and the acceptance criteria. |
| `examples/*.json` | Worked examples across all four regimes (validator-clean). |
| `tests/*.json` | Schema and output contract checks. |

## Key parameters

| Parameter | Value |
|---|---|
| Eligible universe | 20 BSC tokens (USDT reserve) |
| Sizing method | inverse volatility / risk parity (`weight ∝ 1/realised_vol`) |
| `target_portfolio_vol` | 0.45 (annualised), scalar clamped to `[0.20, 1.00]` (no leverage) |
| `max_sleeves` | 12 |
| `max_position_weight_pct` | 17% |
| `target_stable_reserve_pct` | 15% (min 10%) |
| `stop_loss_pct` / ATR backstop | 15% / 3.5x ATR(14) |
| `drawdown_throttle_pct` / `kill_switch_pct` | 22% / 24% |
| Exposure multipliers | breakout 1.0 · risk_on 1.0 · chop 0.8 · risk_off 0.4 |

## Sizing in one line

```
inv_vol_i = 1 / max(realised_vol_i, 0.05)
raw_parity_weight_i = inv_vol_i / Σ inv_vol_j          # equal-risk contribution (risk parity)
                      (crates/portfolio-optimizer::inverse_volatility / risk_parity_lite)
target_vol_scalar = clamp(0.45 / est_book_vol, 0.20, 1.00)   # de-risk when vol spikes; never lever
weight_i = min( 100 * regime_multiplier * target_vol_scalar * raw_parity_weight_i , 17% )   # USDT = remainder
```

## Validate the examples

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/volatility-targeted-risk-parity/examples'); print('valid' if ex and all(V(e)==[] for e in ex) else 'INVALID', len(ex))"
```

## Backtest

```bash
cargo run -p guardrail-cli -- backtest --preset balanced
cargo run -p guardrail-cli -- walk-forward --preset balanced
```

See [`../README.md`](../README.md) for the full skill catalog and
[`../COMPARISON.md`](../COMPARISON.md) for how this Skill complements the others.
