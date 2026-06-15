# social-sentiment-momentum

A Track-2 **Strategy Skill** for Guardrail Alpha: a social + sentiment momentum
strategy over the 20 eligible BSC tokens. It reads the **crowd** instead of the
**chart** — favouring names with **accelerating attention CONFIRMED by money**
(rising CMC trending rank + a volume surge + positive social momentum), **fading
hype without volume**, and **de-risking at sentiment extremes** (extreme-greed
blowoff / extreme-fear capitulation). It scales exposure UP in risk_on/breakout
and cuts hard in chop/risk_off. Signal-diverse to the price-structure Skills
[`trend-breakout-momentum`](../trend-breakout-momentum/) and
[`mean-reversion-chop`](../mean-reversion-chop/).

> This is a strategy **specification**, not an executor. The Rust risk engine is
> the final authority over every trade.

## Files

| File | Purpose |
|---|---|
| `skill.yaml` | Skill manifest — name, inputs, outputs, regimes, example/prompt index. |
| `strategy_spec.yaml` | Machine-readable strategy spec — universe, regime model, attention signal, allocation, risk policy. |
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
| `stop_loss_pct` / `take_profit_pct` | 12% / 22% |
| `drawdown_throttle_pct` / `kill_switch_pct` | 22% / 24% |
| Exposure multipliers | breakout 1.1 · risk_on 1.0 · chop 0.4 · risk_off 0.15 |
| Sentiment-extreme gate | greed >= 80 / fear <= 20 → tilt factor 0.6 |

## Signal in one line

```
attention_tilt = 0.4*trend_score + 0.35*volume_component + 0.25*social_component
  trend_score      — trending-rank velocity (climbing the CMC trending board)
  volume_component — 1.0 if volume_ratio >= 1.5 (confirmed by money); 0 if < 1.0 (hype)
  social_component — positive social momentum (mentions/views accelerating)
sentiment_score = clamp01(base_score adjusted by attention_tilt * sentiment_gate) * (1 - security_penalty)
```

## Validate the examples

```bash
cd python-lab && python3 -c "from guardrail_lab.skill import load_skill_examples as L, validate_example as V; ex=L('../skills/social-sentiment-momentum/examples'); print('valid' if ex and all(V(e)==[] for e in ex) else 'INVALID', len(ex))"
```

## Backtest

```bash
cargo run -p guardrail-cli -- backtest --preset aggressive
cargo run -p guardrail-cli -- walk-forward --preset aggressive
```

See [`../README.md`](../README.md) for the full skill catalog.
