# Track 2 — Strategy Skills

**Submission:** `regime-routed-bsc-alpha` — a backtestable, regime-routed crypto
trading strategy authored as an LLM **Skill** and validated against the same Rust
engine that trades it live.

Deliverable skill: [`skills/cmc-regime-routed-alpha/`](../skills/cmc-regime-routed-alpha)

---

## What it is

A **strategy spec packaged as a Skill** — not a live agent. The Skill is a
portable, declarative artifact (`skill.yaml` + `strategy_spec.yaml` + prompts +
examples + tests) that describes how to turn CoinMarketCap market intelligence
into a regime-routed BSC portfolio. It declares its inputs, its outputs, and the
regime/scoring/sizing logic, so any consumer (an LLM, a backtester, a reviewer)
can reason about and replay the strategy without execution authority.

Critically, the Skill is **advisory only**: its system prompt states it "cannot
execute trades or override Rust risk validation," and `strategy_spec.yaml` pins
`execution_layer: twak_only` and `require_rust_policy_validation: true`. The Skill
proposes; the Rust risk engine disposes. This is what makes it *safe to share* —
the spec can be copied, forked, and backtested by anyone with no path to signing.

## Inputs it consumes (from the CMC AI Agent Hub)

Declared in `skill.yaml`:

| Input | Use |
|---|---|
| `cmc_quotes` | Per-asset price, 24h return, volume — the core scoring input. |
| `cmc_ohlcv` | Candles for momentum / volatility features (RSI, MACD). |
| `cmc_dex_liquidity` | BSC DEX liquidity for the liquidity feature + slippage. |
| `cmc_fear_greed` | Market-wide sentiment input to the regime classifier. |
| `cmc_trending` | Discovery candidates. |
| `eligible_asset_list` | The BSC-eligible universe the strategy may trade. |

These map one-to-one onto the agent's `CmcDataSource` trait, which is served live
over the CMC AI Agent Hub via an MCP (JSON-RPC) transport. See
[CMC_INTEGRATION.md](CMC_INTEGRATION.md).

## Outputs it produces

`market_regime`, `asset_scores`, `target_portfolio`, `entry_rules`,
`exit_rules`, `risk_policy` — the full decision envelope, never a swap.

---

## Strategy logic

### 1. Regime detection

`strategy_spec.yaml` defines four regimes — `risk_on`, `risk_off`, `chop`,
`breakout` — routed from three macro inputs: **Fear & Greed**, **market breadth**
(% of assets advancing), and a **derivatives/funding proxy** (median 24h return,
which stands in for directional pressure). First match wins, and each regime
carries an exposure multiplier:

| Regime | Condition (first match wins) | Exposure |
|---|---|---|
| `breakout` | breadth ≥ 65% AND median > 2% AND F&G ≥ 60 | 1.1× |
| `risk_on` | breadth ≥ 55% AND F&G ≥ 50 | 1.0× |
| `risk_off` | breadth ≤ 40% OR F&G ≤ 30 OR median < -2% | 0.2× |
| `chop` | everything else (directionless) | 0.5× |

### 2. Signal blend (per the brief's RSI / MACD / F&G example)

Each non-stable asset is reduced to normalized 0..1 feature scores and blended
into one alpha score. The blend covers **momentum** (RSI/MACD-style trend),
**volume acceleration**, **liquidity**, **volatility**, **execution quality**, and
**sentiment (F&G)** — exactly the RSI + MACD + Fear & Greed family the Track 2
brief uses as its worked example:

| Feature | Weight |
|---|---|
| momentum (RSI / MACD) | 0.30 |
| execution quality | 0.20 |
| volume acceleration | 0.15 |
| liquidity | 0.15 |
| volatility | 0.10 |
| sentiment (F&G) | 0.10 |

A token-security **risk penalty** is applied as a multiplicative haircut
(`score = normalized * (1 - risk_penalty)`), so unsafe tokens are demoted even
when their technicals look strong.

### 3. Entry / exit / sizing rules

- **Entry:** select assets with `score ≥ min_score_to_enter` (0.65), capped at
  `max_positions` (5).
- **Sizing:** risk budget = `(100 − stable_reserve_pct) × regime_multiplier`;
  each name gets a score-proportional slice, hard-capped at
  `max_position_weight_pct` (17%). Surplus and the remainder fall to a `USDT`
  reserve. Down regimes (`risk_off` 0.2×, `chop` 0.5×) automatically de-risk into
  the reserve — the capital-preservation behavior.
- **Exit:** rebalance only when weight drift exceeds a 3% no-churn band; force an
  exit when conviction drops below `min_score_to_hold` (0.50); fully exit any held
  asset no longer in the target set. All risk-asset trades route through `USDT`.

Worked regime outputs ship as examples:
- [`examples/risk_on_example.json`](../skills/cmc-regime-routed-alpha/examples/risk_on_example.json) — tilts into `CAKE`.
- [`examples/chop_example.json`](../skills/cmc-regime-routed-alpha/examples/chop_example.json) — 70% `USDT`.
- [`examples/risk_off_example.json`](../skills/cmc-regime-routed-alpha/examples/risk_off_example.json) — 90% `USDT`.

Full prose: [STRATEGY.md](STRATEGY.md).

---

## How it is backtested

The Skill is **defensibly backtestable** because the strategy spec mirrors the
production Rust pipeline. The backtester re-runs the *exact* live
`strategy → risk → portfolio` code path over a deterministic synthetic market
path, so research and live trading share one implementation — no separate,
diverging backtest model.

| Tool | What it does |
|---|---|
| `guardrail-cli backtest / walk-forward / sweep` | Single run, ramped multi-window run, and a Fear & Greed parameter sweep over the real engine. |
| `guardrail-sim` | Sentiment sweep / walk-forward binary over the same backtest engine. |
| `python-lab` experiment tracking | The CLI writes one JSON per run to `data/experiments/<tag>.json`; `guardrail_lab/experiments.py` + `scripts/export_experiments.py` load and compare runs (stdlib-only). |
| Skill schema tests | [`tests/test_strategy_schema.json`](../skills/cmc-regime-routed-alpha/tests/test_strategy_schema.json) asserts the required output keys; [`tests/test_outputs.json`](../skills/cmc-regime-routed-alpha/tests/test_outputs.json) is a conforming sample, keeping the Skill's contract checkable. |

Every run is scored against an **equal-weight buy-and-hold** benchmark over the
non-stable universe; `excess_return_pct` is the strategy's alpha. Reported
metrics: `total_return_pct`, `max_drawdown_pct`, `trade_count`, `win_rate_pct`,
`profit_factor`, `volatility_pct`, `calmar_ratio`. Fills include slippage (price
impact + venue spread) and flat BSC gas. The sweep is the headline visual: it
shows the regime router cutting exposure as F&G falls from greedy to fearful.

> Note: the synthetic path is a reproducible **model**, not realized history — it
> measures how the strategy behaves under a stylized regime. Full methodology:
> [BACKTEST_METHODOLOGY.md](BACKTEST_METHODOLOGY.md).

---

## Deliverables

```
skills/cmc-regime-routed-alpha/
├── skill.yaml              # name: regime-routed-bsc-alpha; inputs + outputs
├── strategy_spec.yaml      # universe, 4 regimes, risk: twak_only + rust validation
├── README.md
├── prompts/
│   ├── system.md           # advisory-only role; cannot execute or override risk
│   ├── strategy_generation.md
│   └── backtest_spec.md
├── examples/               # risk_on / risk_off / chop worked outputs
│   ├── risk_on_example.json
│   ├── risk_off_example.json
│   └── chop_example.json
└── tests/                  # output-schema contract + conforming sample
    ├── test_strategy_schema.json
    └── test_outputs.json
```

The same descriptor is also served live at `GET /skill` and rendered on the
dashboard `/skill` page.

---

## Mapping to Track-2 judging criteria

| Criterion | How this submission scores |
|---|---|
| **Technical execution** | The Skill spec mirrors a real Rust pipeline; the backtester reuses the *production* `StrategyEngine`/`RiskEngine`/`PortfolioState`, so backtest and live share one code path. Three replay modes (backtest / walk-forward / sweep), a benchmark-relative alpha metric, slippage+gas fills, and schema tests on the Skill's own contract. |
| **Originality** | A strategy authored *as a Skill*, not as a bespoke bot: regime routing across four regimes (incl. `breakout`), a security-penalty haircut on alpha, and a hard authority boundary (advisory Skill, Rust-only execution) that makes the strategy safe to share and fork. |
| **Real-world relevance** | Consumes live CMC AI Agent Hub data (quotes, OHLCV, F&G, DEX liquidity, trending) over the BSC-eligible universe; the RSI/MACD/F&G blend and de-risk-in-downturns sizing are exactly what a discretionary desk does, encoded declaratively. |
| **Demo** | One-command reproducible runs: `guardrail-cli backtest \| walk-forward \| sweep`, `guardrail-sim`, `python-lab/scripts/export_experiments.py`, and the `/skill` + `/backtest` + `/sweep` API routes and dashboard pages. Deterministic mocks mean the demo runs fully offline. |

---

## Verify

```bash
# Inspect the Skill artifact
cat skills/cmc-regime-routed-alpha/skill.yaml
cat skills/cmc-regime-routed-alpha/strategy_spec.yaml

# Backtest the strategy over the real engine
cargo run -p guardrail-cli -- backtest      --config configs/paper.toml
cargo run -p guardrail-cli -- walk-forward  --config configs/paper.toml
cargo run -p guardrail-cli -- sweep         --config configs/paper.toml

# Compare tracked experiments (stdlib-only)
python3 python-lab/scripts/export_experiments.py
```

Related: [STRATEGY.md](STRATEGY.md) · [BACKTEST_METHODOLOGY.md](BACKTEST_METHODOLOGY.md) · [CMC_INTEGRATION.md](CMC_INTEGRATION.md) · [HACKATHON.md](HACKATHON.md)
