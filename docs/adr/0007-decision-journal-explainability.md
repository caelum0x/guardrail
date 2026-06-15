# ADR 0007 — Decision journal as the explainability projection

- Status: Accepted
- Date: 2026-06-14

## Context

Autonomy is only acceptable when it is **verifiable**: a third party must be
able to reconstruct *why the agent did what it did* without trusting the agent's
own after-the-fact narration. We already persist an append-only event log
(ADR 0004), but raw rows are not human-readable, and any narrative the agent
generates separately would be an unverifiable claim.

## Decision

The append-only event log (`data/guardrail_alpha.db`, `events` table) is the
**single source of truth** for explainability. The decision journal
(`python-lab/guardrail_lab/journal.py`, exposed as
`python3 python-lab/analyze.py journal`) is a **pure, deterministic projection**
of that log — it adds no information the log does not already contain.

The renderer segments the event stream into cycles at each `regime_classified`
event and, per cycle, narrates: the regime, the top scored assets, the proposed
orders, the risk engine's verdict (`risk_approved` / `risk_clipped` /
`risk_rejected`, with rejection reasons), the confirmed trades, and the
reconciled NAV. Crucially, the **risk verdict is logged separately from the
proposal**, so a refusal to trade is a first-class, auditable event — negative
space is on the record, not invisible.

## Consequences

- Anyone can re-run the deterministic renderer over the same `events` table and
  get the identical narrative; the journal is a faithful projection, not an
  independent assertion. Write it out with `--out data/journal.md`.
- Explainability requires no live state or network: the committed log plus the
  strategy specs and `skills/ensemble.json` re-derive both *proposed*
  allocations (the `ensemble` subcommand) and *executed* decisions (the
  `journal` subcommand) from the repository alone.
- The journal degrades gracefully: an empty log yields a clear "no data" note
  (exit 0), never an error.
- Trade-off: the journal can only ever be as complete as the emitted events;
  any decision worth explaining must emit a typed event at the time it is made.
- See `docs/EXPLAINABILITY.md` for the full event-to-narrative mapping.
