# trend-breakout-momentum

A Track-2 **Strategy Skill** for Guardrail Alpha: a momentum / breakout strategy
specialised for the **breakout** regime over the 20 eligible BSC tokens. It
enters on confirmed, volume-backed upside breakouts with an aligned trend and
rising momentum, rides them with ATR trailing stops, and scales exposure UP in
breakout/risk_on while cutting hard in chop/risk_off. It is the mirror image of
[`mean-reversion-chop`](../mean-reversion-chop/).

> This is a strategy **specification**, not an executor. The Rust risk engine is
> the final authority over every trade.

## Files

| File | Purpose |
|---|---|
| `skill.yaml` | Skill manifest — name, inputs, outputs, regimes, example/prompt index. |
| `strategy_spec.yaml` | Machine-readable strategy spec — universe, regime model, breakout signal, allocation, risk policy. |
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
| Exposure multipliers | breakout 1.1 · risk_on 1.0 · chop 0.4 · risk_off 0.15 |

## Signal in one line

```
breakout_tilt = 0.4*trend_score + 0.35*momentum_component + 0.25*struct_score
  trend_score        — EMA12 > EMA26 > EMA50, rising, price above stack
  momentum_component — MACD histogram positive and expanding
  struct_score       — 1.0 if close > donchian_upper(20) AND volume_ratio >= 1.5; 0 if volume_ratio < 1.0
breakout_score = clamp01(base_score adjusted by breakout_tilt) * (1 - security_penalty)
```

## Validate the examples

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/trend-breakout-momentum/examples'); print('valid' if ex and all(V(e)==[] for e in ex) else 'INVALID', len(ex))"
```

## Backtest

```bash
cargo run -p guardrail-cli -- backtest --preset aggressive
cargo run -p guardrail-cli -- walk-forward --preset aggressive
```

See [`../README.md`](../README.md) for the full skill catalog and
[`../COMPARISON.md`](../COMPARISON.md) for how this Skill complements the others.
