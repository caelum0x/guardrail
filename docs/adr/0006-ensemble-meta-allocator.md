# ADR 0006 — Ensemble meta-allocator above the strategy skills

- Status: Accepted
- Date: 2026-06-14

## Context

Track 2 ships four single-thesis strategy skills (trend/breakout momentum,
CMC regime-routed alpha, funding-rate carry, mean-reversion chop). Each is
strong only in its home regime and weak elsewhere. Picking one statically
leaves performance on the table and makes regime transitions brittle; baking a
"pick the right skill" rule into the Rust engine would couple alpha selection to
the safety-critical execution gate, which must stay small and auditable.

## Decision

Put a **regime-routed meta-allocator** in the Python/config layer
(`python-lab/guardrail_lab/ensemble.py` + `skills/ensemble.json`), *above* the
four skills and *below* the risk engine. Given the regime classified by the
strategy engine, it looks up per-regime blend weights (summing to 1.0), pulls
each skill's example target portfolio for that regime, takes a weighted average
of the per-symbol risk weights, renormalizes the risk total to
`<= max_risk_allocation_pct` (100), and holds the single remainder as one USDT
reserve line (never summing skills' individual reserves).

The ensemble owns **no alpha of its own** — it is a router. Its output is an
**advisory target book**, not an order. It is pure, standard-library-only, and
fail-soft: missing/malformed inputs yield a clearly-empty result with a
human-readable reason. The Rust risk engine (ADR 0002) remains the sole
execution authority and independently re-checks every blended position.

## Consequences

- The blended book is a pure function of `(regime, ensemble.json, the four
  example files)`, so any reviewer can re-derive it offline:
  `python3 python-lab/analyze.py ensemble --regime breakout`.
- Alpha selection evolves in config/Python without touching or recompiling the
  Rust gate; the core safety invariant (one independent risk authority) holds
  regardless of how many strategies blend into a proposal.
- Trade-off: the blend is only as good as the committed example portfolios; it
  cannot invent signal beyond what the skills express for each regime.
- See `docs/ENSEMBLE.md` for the weight table and the diversification rationale.
