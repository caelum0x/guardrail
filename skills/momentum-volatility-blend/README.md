# momentum-volatility-blend

A Track-2 **Strategy Skill** for Guardrail Alpha: a strategy that BLENDS price
momentum with a **volatility-quality filter** over the 20 eligible BSC tokens. It
favours names that are simultaneously strongly trending AND sitting in a healthy
realised-volatility band, demoting both **dead-vol** names (no trend energy, the
move stalls) and **blow-off-vol** names (parabolic, exhaustion-prone, they
mean-revert). It rides healthy-vol winners with ATR stops and scales exposure UP
in breakout/risk_on while cutting hard in chop/risk_off. It complements
[`trend-breakout-momentum`](../trend-breakout-momentum/) (pure breakout),
[`mean-reversion-chop`](../mean-reversion-chop/) (range-fade) and
[`volatility-targeted-risk-parity`](../volatility-targeted-risk-parity/) (pure
sizing).

> This is a strategy **specification**, not an executor. The Rust risk engine is
> the final authority over every trade.

## Files

| File | Purpose |
|---|---|
| `skill.yaml` | Skill manifest — name, inputs, outputs, regimes, example/prompt index. |
| `strategy_spec.yaml` | Machine-readable strategy spec — universe, regime model, blend signal, allocation, risk policy. |
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
| `min_score_to_enter` / `min_score_to_hold` | 0.65 / 0.50 |
| `max_positions` | 5 |
| `max_position_weight_pct` | 17% |
| `target_stable_reserve_pct` | 12% (min 10%) |
| `stop_loss_pct` / `take_profit_pct` | 12% / 40% |
| `drawdown_throttle_pct` / `kill_switch_pct` | 22% / 24% |
| Vol band (annualised) | dead < 40 · healthy 50-80 · blow-off > 110 |
| Exposure multipliers | breakout 1.1 · risk_on 1.0 · chop 0.4 · risk_off 0.15 |

## Signal in one line

```
momentum_leg = 0.5*trend_score + 0.5*momentum_score   (0 in downtrend / negative MACD hist)
vol_quality  = bell over realised_vol: 1.0 in 50-80, ramp to 0 at floor 40 / ceiling 110
blend_tilt   = momentum_leg * vol_quality              (multiplicative — both legs required)
blend_score  = clamp01(base_score adjusted by blend_tilt) * (1 - security_penalty)
```

## Validate the examples

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/momentum-volatility-blend/examples'); print('valid' if ex and all(V(e)==[] for e in ex) else 'INVALID', len(ex))"
```

## Backtest

```bash
cargo run -p guardrail-cli -- backtest --preset aggressive
cargo run -p guardrail-cli -- walk-forward --preset aggressive
```

See [`../README.md`](../README.md) for the full skill catalog and
[`../COMPARISON.md`](../COMPARISON.md) for how this Skill complements the others.
