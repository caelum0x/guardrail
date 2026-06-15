# Track-2 Strategy Skills — Catalog

This directory is the **registry of Track-2 (Strategy Skills)** for Guardrail. A
Track-2 skill is a *backtestable trading-strategy specification* expressed as
data + prompts, **not** an executor. Each skill turns CoinMarketCap (and
Guardrail-derived) market data into a regime-routed long/stable rotation over the
same fixed universe of 20 eligible BSC tokens, emits a decision payload (regime,
per-asset scores, target portfolio, entry/exit/heartbeat actions), and defers the
final word on every trade to the production Rust risk engine.

> A skill describes *what* the strategy would do. The Rust risk engine in
> `crates/risk-engine` remains the **final authority** — it validates, clamps, or
> rejects every LLM-proposed target before a swap ever happens. A skill can
> re-rank and re-size candidates, but it can never breach a risk limit.

## The skills

| Skill | Thesis | Regimes | Examples | Directory |
|-------|--------|---------|----------|-----------|
| **regime-routed-bsc-alpha** | Route exposure by market regime, then rank assets with a multi-factor alpha blend (momentum + RSI/MACD + volume + volatility + liquidity + sentiment + execution-quality, minus a security penalty). | risk_on, risk_off, chop, breakout | 4 | [`cmc-regime-routed-alpha/`](./cmc-regime-routed-alpha) |
| **funding-rate-carry-bsc** | Same regime routing + risk envelope, but layer a **funding-carry tilt** on top of the base alpha score: prefer assets the perp crowd is under-positioned in (low/negative funding), fade richly-positive crowded longs. | risk_on, risk_off, chop, breakout | 4 | [`funding-rate-carry/`](./funding-rate-carry) |
| **mean-reversion-chop-bsc** | Same regime classifier + risk envelope, but a **counter-trend reversion tilt**: buy oversold dips (low RSI / lower-Bollinger-band touches), trim overbought stretches, hold a large reserve. **Inverted** exposure profile — most active in chop, steps aside in trending risk_on/breakout, minimal in risk_off. | risk_on, risk_off, chop, breakout | 4 | [`mean-reversion-chop/`](./mean-reversion-chop) |
| **trend-breakout-momentum-bsc** | Same regime classifier + risk envelope, but a **momentum / breakout tilt**: enter only confirmed, volume-backed upside breakouts (aligned rising EMA stack + positive MACD histogram + close above the 20-bar Donchian high with `volume_ratio >= 1.5`), ride with ATR trailing stops. Exposure **peaks in breakout** (1.1) and risk_on, cut hard in chop/risk_off — the mirror image of mean-reversion-chop. | risk_on, risk_off, chop, breakout | 4 | [`trend-breakout-momentum/`](./trend-breakout-momentum) |
| **volatility-targeted-risk-parity-bsc** | A **new axis** — risk-based *sizing*, not signal direction. Instead of deciding *what* to buy, it decides *how much*: size each name by the **inverse of its realised volatility** so every holding contributes ~equal risk (**risk parity**, mirroring `crates/portfolio-optimizer::inverse_volatility` / `risk_parity_lite`), then **scale gross to a 45% target portfolio volatility** (`target_vol_scalar = clamp(0.45/est_book_vol, 0.20, 1.00)`, never levered). **De-risks in risk_off** as vol spikes. Per-name cap 17%, USDT reserve remainder. Registered as an **additional standalone strategy** (the four-skill ensemble core in `ensemble.json` is unchanged). | risk_on, risk_off, chop, breakout | 4 | [`volatility-targeted-risk-parity/`](./volatility-targeted-risk-parity) |
| **social-sentiment-momentum-bsc** | Same regime classifier + risk envelope, but a **social + sentiment attention tilt**: read the crowd, not the chart. Favour names with **accelerating attention CONFIRMED by money** — rising CMC trending-rank velocity + a volume surge (`volume_ratio >= 1.5`) + positive social momentum (`attention_tilt = 0.4*trend_score + 0.35*volume_component + 0.25*social_component`) — **fade hype without volume** (`volume_ratio < 1.0`), and **de-risk at sentiment extremes** via a Fear & Greed gate (factor 0.6 at >= 80 blowoff / <= 20 capitulation). Exposure full/peak in risk_on/breakout, cut in chop/risk_off. | risk_on, risk_off, chop, breakout | 4 | [`social-sentiment-momentum/`](./social-sentiment-momentum) |

The machine-readable version of this table lives in
[`INDEX.json`](./INDEX.json) — one entry per skill with `id`, `name`, `path`,
`summary`, `regimes`, `inputs`, `eligible_universe_size`, `examples_count`, and
`spec_file`.

## The shared regime model

Every skill classifies the market into one of four regimes using the same
top-down rules (mirroring `crates/strategy-engine/src/regime.rs::classify()`).
The first matching rule wins:

| Order | Regime | Condition | Exposure multiplier |
|-------|--------|-----------|---------------------|
| 1 | `breakout` | `breadth_pct >= 65 AND median_24h_return > 2 AND fear_greed >= 60` | 1.1 |
| 2 | `risk_on` | `breadth_pct >= 55 AND fear_greed >= 50` | 1.0 |
| 3 | `risk_off` | `breadth_pct <= 40 OR fear_greed <= 30 OR median_24h_return < -2` | 0.2 |
| 4 | `chop` | default (none of the above) | 0.5 |

Inputs: `breadth_pct` (% of risk assets with a positive 24h return),
`median_24h_return`, and the CMC Fear & Greed value (0..100). The exposure
multiplier scales the risk budget in the allocator.

## The shared risk envelope

All skills honour the same risk envelope, sourced from
`crates/risk-engine/src/policy.rs` and
`crates/strategy-engine/src/strategy_config.rs`:

- Per-name cap **17%** (policy max **18%**); stable reserve **15% target**
  (>= **10%** floor). Surplus over the cap falls back to the USDT reserve — never
  rejected.
- `min_score_to_enter` **0.65**, `min_score_to_hold` **0.50**,
  `max_positions` **5**, `max_new_position_pct` **12%**.
- Stop-loss **12%**, take-profit **25%** per position; rebalance only when target
  vs current drifts > **3%** of NAV.
- Drawdown throttle at **22%** total drawdown (block new buys, soft);
  kill switch latches at **24%** (halt trading).
- Execution: `require_quote_before_swap`, `twap_only`, `max_slippage_pct` **0.8**.
- Daily-trade requirement: >= **1** trade/day (a heartbeat <= **0.10%** NAV
  satisfies it when the book is flat).

## How `python-lab/guardrail_lab/skill.py` validates examples

The validator (`python-lab/guardrail_lab/skill.py`) is standard-library only and
shape-tolerant. Two functions matter:

- `load_skill_examples(skill_dir)` loads every `*.json` file in a skill's
  `examples/` directory, annotates each with a `_source` key (the file name),
  skips anything that is not a JSON object, and returns the list sorted by file
  name.
- `validate_example(example)` returns a **list of human-readable issues**, or an
  empty list `[]` when the example is well-formed.

`validate_example` searches the top level **and** the nested `computed`,
`decision`, and `inputs` scopes, so a full signal->decision payload validates the
same as a minimal `{ "market_regime": ..., "target_portfolio": [...] }` example.
It checks that:

1. a market regime is present and non-empty (`market_regime` or `regime`);
2. a target portfolio is present, a non-empty list, with each position carrying a
   `symbol` and a numeric `weight_pct` (`target_portfolio` or `portfolio`);
3. portfolio risk weights do **not** over-allocate (sum <= 100 within a 1.0pp
   tolerance — the remainder is the held stable reserve, so sums below 100 are
   expected and fine);
4. if (and only if) the example declares a trade-rules block, both `entry` and
   `exit` are present (nested `"rules": {...}` or flat `entry`/`exit`).

Run the validator over both skills:

```bash
cd python-lab && python3 -c "
from guardrail_lab.skill import load_skill_examples, validate_example
for d in ['../skills/cmc-regime-routed-alpha/examples',
          '../skills/funding-rate-carry/examples',
          '../skills/mean-reversion-chop/examples',
          '../skills/trend-breakout-momentum/examples',
          '../skills/volatility-targeted-risk-parity/examples',
          '../skills/social-sentiment-momentum/examples']:
    ex = load_skill_examples(d)
    assert ex and all(validate_example(e) == [] for e in ex), d
    print(d, 'ok', len(ex), 'examples')
"
```

## How to add a new skill

1. Create a new top-level directory under `skills/` (e.g.
   `skills/my-new-strategy/`).
2. Provide the required file layout:

   ```
   my-new-strategy/
   ├── skill.yaml              # manifest: name, version, track, inputs, outputs, regimes, examples
   ├── strategy_spec.yaml      # the complete, backtestable strategy spec (single source of truth)
   ├── README.md               # quick-start summary
   ├── SKILL.md                # (optional) longer narrative + decision procedure
   ├── prompts/
   │   ├── system.md           # role + hard constraints for the strategy LLM
   │   ├── strategy_generation.md  # step-by-step regeneration recipe
   │   └── backtest_spec.md    # how to produce a defensible backtest
   ├── examples/               # one full signal -> decision payload per regime
   │   ├── risk_on_example.json
   │   ├── risk_off_example.json
   │   ├── chop_example.json
   │   └── breakout_example.json
   └── tests/                  # required-output schema + smoke fixtures
   ```

3. Honour the shared regime model and risk envelope above (or document any
   deliberate deviation in `strategy_spec.yaml`).
4. Make every example satisfy the **`validate_example` contract**: each example
   must produce `[]` from `validate_example`. Concretely, ensure each example
   carries a non-empty `market_regime`, a non-empty `target_portfolio` list whose
   positions each have a `symbol` and numeric `weight_pct`, and whose risk weights
   sum to <= 100 (leave the remainder as the USDT reserve). If you add a
   `rules`/`entry`/`exit` block, include both sides.
5. Add an entry to [`INDEX.json`](./INDEX.json) and a row to the table above,
   pulling all values from your real spec files (and counting the
   `examples/*.json` files for `examples_count`).
6. Verify:

   ```bash
   python3 -c "import json; d=json.load(open('skills/INDEX.json')); assert len(d)>=2; print('index ok')"
   ```

   then re-run the validator snippet above to prove the catalog points at valid
   skills.
