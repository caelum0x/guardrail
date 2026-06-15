# Regime Ensemble (meta-allocator)

**Thesis.** Don't run one thesis in every market. The four Track-2 strategy
skills are deliberately complementary — momentum buys strength, mean-reversion
buys weakness, carry is orthogonal, and the general regime-routed alpha is the
backbone — so blend them by the *currently classified regime* and let the right
specialist lead where it was built to win. This is a meta-strategy: it routes
between the skills instead of replacing them.

The blend weights live in [`skills/ensemble.json`](../../skills/ensemble.json)
(the authoritative source) and are mirrored, with full per-regime rationale, in
[`skills/ENSEMBLE.md`](../../skills/ENSEMBLE.md).

## The four skills it blends

| Skill | Home regime | Thesis |
|---|---|---|
| `trend-breakout-momentum` | breakout / risk_on | Buys confirmed strength (EMA stack, MACD, Donchian breakout on volume). |
| `cmc-regime-routed-alpha` | all (general) | Multi-factor alpha backbone; rotates fully to USDT in risk_off. |
| `funding-rate-carry` | risk_on / breakout | Leans into cheaply-funded longs; fades crowded ones. |
| `mean-reversion-chop` | chop | Fades oversold dips, trims overbought rips — pays in ranges, bleeds in trends. |

## Per-regime blend weights

Each row sums to 1.0. These are the exact values from `skills/ensemble.json`
(`regimes.<regime>.weights`).

| Regime | trend-breakout | regime-alpha | funding-carry | mean-reversion |
|---|---:|---:|---:|---:|
| **risk_on** | 0.35 | 0.35 | 0.25 | 0.05 |
| **risk_off** | 0.10 | 0.45 | 0.35 | 0.10 |
| **chop** | 0.08 | 0.30 | 0.12 | 0.50 |
| **breakout** | 0.50 | 0.30 | 0.15 | 0.05 |

- **risk_on** — trend + general alpha co-lead a broad up-tape; carry adds a tilt;
  reversion is suppressed because fading strength bleeds in a trend.
- **risk_off** — capital preservation dominates: the general alpha rotates to
  USDT and carry de-risks regardless of funding, so both carry the most weight.
- **chop** — mean-reversion is in its home regime (0.50); the general alpha is
  the steady backbone; trend and carry are cut hard (breakouts whipsaw, carry is
  noisy without drift).
- **breakout** — trend/breakout momentum peaks (0.50, 1.1x exposure) and leads;
  the general alpha adds breadth across the strongest names.

The blend is intentionally smooth: even a suppressed skill keeps a small weight,
so transitions between regimes don't whipsaw the book.

## How the blend is computed

For a classified regime, each skill contributes its **example target portfolio**
for that regime
(`skills/<skill>/examples/<regime>_example.json` → `decision.target_portfolio`,
a list of `{symbol, weight_pct}`). The meta-allocator:

1. drops each skill's own USDT reserve line and keeps only **risk** positions;
2. for each symbol, computes the blended risk weight
   `Σ blend_weight[skill] × skill_weight_pct[symbol]`;
3. renormalizes total risk weight to `≤ max_risk_allocation_pct` (100);
4. holds the remainder as a single **USDT reserve** line.

A skill whose example is missing simply contributes nothing (`Loaded = no`) and
the blend degrades gracefully — it never errors out.

## Run it (offline, deterministic)

```bash
# Blend for the current regime in the event log (falls back to risk_on):
python3 python-lab/analyze.py ensemble

# Blend a specific regime (risk_on | risk_off | chop | breakout):
python3 python-lab/analyze.py ensemble --regime chop

# Same, via the single dispatcher:
scripts/guardrail.sh analyze ensemble --regime breakout

# Compare the blended book vs. each single skill
# (concentration / diversification / overlap), per regime:
python3 python-lab/analyze.py ensemble-compare --regime risk_on
```

`analyze.py ensemble` prints two tables: the **Blended Target Portfolio** (risk
names first, USDT reserve last) and the **Per-Skill Contribution** (each skill's
blend weight for the regime, its own deployed risk %, the % it contributed, and
whether its example loaded).

## The risk engine is still the only gate

This blended book is **advisory**. It is handed to the Rust risk engine, which
enforces per-name caps, the stable-reserve floor, throttling, and the drawdown
kill-switch — and may clip or reject any position. The ensemble proposes; the
engine disposes.

## See also

- [`skills/ENSEMBLE.md`](../../skills/ENSEMBLE.md) — full rationale and attribution guide.
- [`skills/ensemble.json`](../../skills/ensemble.json) — authoritative blend weights.
- [README.md](./README.md) — the single-thesis strategy cookbook these skills complement.
