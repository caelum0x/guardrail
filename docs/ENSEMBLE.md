# Ensemble Architecture — Regime-Routed Meta-Allocator

This document describes how the **ensemble meta-allocator** sits above the four
Track-2 strategy skills and why it still defers to the Rust risk engine as the
sole execution gate.

## Where the ensemble sits

```
                        ┌──────────────────────────────────────┐
   market data ───────► │  regime classifier (strategy-engine) │
                        │  -> risk_on | risk_off | chop |       │
                        │     breakout                          │
                        └───────────────┬──────────────────────┘
                                        │ regime
                                        ▼
        ┌───────────────────────────────────────────────────────────┐
        │  ENSEMBLE META-ALLOCATOR  (python-lab/guardrail_lab/        │
        │                            ensemble.py + skills/ensemble.json)│
        │                                                             │
        │   per-regime blend weights (sum = 1.0)                      │
        │        ├─ trend-breakout-momentum                           │
        │        ├─ cmc-regime-routed-alpha                           │
        │        ├─ funding-rate-carry                                │
        │        └─ mean-reversion-chop                               │
        │                                                             │
        │   each skill -> its example target_portfolio for the regime │
        │   weighted-average of per-symbol weights -> blended book    │
        │   renormalize risk to <= 100, remainder = USDT reserve      │
        └───────────────────────────┬───────────────────────────────┘
                                     │ proposed target book (advisory)
                                     ▼
        ┌───────────────────────────────────────────────────────────┐
        │  RUST RISK ENGINE  (crates/risk-engine)  — SOLE GATE        │
        │   per-name cap · stable-reserve floor · throttle ·          │
        │   max-drawdown · kill-switch                                │
        │   -> approve | clip | reject  (the only thing that executes)│
        └───────────────────────────────────────────────────────────┘
```

## Design principles

### 1. The ensemble is a *router*, not a strategy
It owns no alpha signal of its own. Every position it proposes traces back to a
named skill's documented thesis and that skill's example portfolio for the
classified regime. This keeps the system auditable: the blended book is a pure
function of `(regime, ensemble.json, the four example files)`.

### 2. Regime-complementary blending
The skills are mirror images across regimes (momentum buys strength,
mean-reversion buys weakness, carry is orthogonal). The blend weights are set so
the specialist whose home regime is active leads, while suppressed skills retain
a small weight to keep the book stable through regime transitions. See
`skills/ENSEMBLE.md` for the weight table and rationale.

### 3. Weighted average, single coherent reserve
The blend takes a weighted average of each skill's *risk* (non-reserve) weights,
then renormalizes the risk total to `≤ max_risk_allocation_pct` (100) and holds
the remainder in **one** USDT reserve line. It never sums skills' individual
reserve lines (which would double-count cash); the reserve is always the
recomputed remainder.

### 4. Pure, fail-soft computation
`ensemble.py` is standard-library only and never raises on missing or malformed
inputs. A missing config, an unconfigured regime, or an absent example file
yields a clearly-empty `EnsembleResult` carrying a human-readable `reason`, and
per-skill attribution records which inputs loaded.

### 5. The risk engine is the only execution authority
The blended book is **advisory**. It is a *target*, not an order. The Rust risk
engine independently re-checks every position against the live policy
(`crates/risk-engine`): per-name caps, the minimum stable reserve, throttling,
the maximum-drawdown limit, and the kill-switch. It may clip a weight, reject a
position, or halt trading entirely. Nothing the ensemble proposes executes
without passing that gate. This preserves the project's core safety invariant:
**a single, independent risk authority guards every trade**, regardless of which
strategy (or blend of strategies) proposed it.

## Verifiability

The blend is reproducible offline from the repository alone:

```bash
python3 python-lab/analyze.py ensemble --regime breakout
```

Because the inputs are committed JSON files and the computation is pure, any
reviewer can re-derive the exact blended book and per-skill attribution for any
regime. Combined with the append-only event log (see
`docs/EXPLAINABILITY.md`), this gives an end-to-end auditable path from market
data to proposed allocation to executed trade.
