# Regime Ensemble — Meta-Strategy

The **ensemble** is a meta-allocator that blends the four Track-2 strategy
skills into a single target portfolio, weighted by the *currently classified
market regime*. It does not replace any skill; it routes between them so the
right specialist leads in the regime it is built for.

## Why blend regime-complementary skills?

The four skills are deliberately **complementary across regimes** — each peaks
where the others step aside:

| Skill | Home regime | Thesis |
| --- | --- | --- |
| `trend-breakout-momentum` | **breakout** / risk_on | Buys confirmed strength (EMA stack, MACD, Donchian breakout on volume). |
| `cmc-regime-routed-alpha` | all (general) | Multi-factor alpha backbone; rotates fully to USDT in risk_off. |
| `funding-rate-carry` | risk_on / breakout | Leans into cheaply-funded longs; fades crowded ones. |
| `mean-reversion-chop` | **chop** | Fades oversold dips, trims overbought rips — pays in ranges, bleeds in trends. |

Because momentum and mean-reversion are *mirror images* (one buys strength, the
other buys weakness), and carry is orthogonal to both, blending them by regime
captures the strategy that historically works in each environment instead of
running one thesis everywhere. The blend is smooth: even in a regime where a
skill is suppressed it keeps a small weight, so transitions between regimes do
not whipsaw the book.

## The weight table

Per-regime blend weights (each row sums to 1.0). See `skills/ensemble.json` for
the authoritative values and the per-regime rationale.

| Regime | trend-breakout | regime-alpha | funding-carry | mean-reversion |
| --- | ---: | ---: | ---: | ---: |
| **risk_on** | 0.35 | 0.35 | 0.25 | 0.05 |
| **risk_off** | 0.10 | 0.45 | 0.35 | 0.10 |
| **chop** | 0.08 | 0.30 | 0.12 | 0.50 |
| **breakout** | 0.50 | 0.30 | 0.15 | 0.05 |

- **risk_on** — trend + general alpha co-lead a broad up-tape; carry adds a tilt; reversion suppressed.
- **risk_off** — capital preservation dominates (alpha rotates to USDT, carry de-risks regardless of funding).
- **chop** — mean-reversion is home (0.50); alpha is the steady backbone; trend/carry cut hard.
- **breakout** — trend/breakout peaks (0.50) and leads; alpha adds breadth.

## How the blend is computed

For a classified regime, each skill contributes its **example target portfolio**
for that regime (`skills/<skill>/examples/<regime>_example.json`, the
`decision.target_portfolio` list of `{symbol, weight_pct}`). The ensemble:

1. drops each skill's own reserve (USDT) line and keeps only **risk** positions;
2. for each symbol, computes the blended risk weight
   `Σ blend_weight[skill] × skill_weight_pct[symbol]`;
3. renormalizes the total risk weight to `≤ max_risk_allocation_pct` (100);
4. holds the remainder as a single **USDT reserve** line.

## Reading the attribution

`python3 python-lab/analyze.py ensemble --regime chop` prints two tables:

- **Blended Target Portfolio** — the final book (risk names first, USDT reserve last).
- **Per-Skill Contribution** — for each skill: its `Blend wt` (this regime's
  weight), its own `Skill risk %` (how much risk its example deployed),
  the `Contributed %` it pushed into the blend (`blend_wt × skill_risk`), and
  whether its example `Loaded`. A skill whose example is missing shows
  `Loaded = no` and contributes nothing — the blend degrades gracefully.

## The risk engine is still the only gate

This blended book is **advisory**. It is handed to the Rust risk engine, which
enforces per-name caps, the stable-reserve floor, throttling, and the drawdown
kill-switch, and may clip or reject any position. The ensemble proposes; the
engine disposes. See `docs/ENSEMBLE.md` for the architecture.
