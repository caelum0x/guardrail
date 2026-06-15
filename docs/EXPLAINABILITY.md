# Explainability & Verifiable Autonomy

Guardrail Alpha is an autonomous trading agent, and autonomy is only acceptable
when it is **verifiable**: a third party must be able to reconstruct *why the
agent did what it did* from a tamper-evident record, without trusting the
agent's own after-the-fact narration. This document describes how that property
is achieved and how to reproduce it.

## The append-only event log is the source of truth

Every meaningful decision the agent makes emits a typed event into an
append-only log (`data/guardrail_alpha.db`, the `events` table). The agent never
mutates or deletes events; it only appends. Each row carries:

- `id` — monotonic insertion order,
- `run_id` — which agent run produced it,
- `timestamp` — ISO-8601 UTC,
- `event_type` — the kind of decision/observation, and
- `payload_json` — the structured detail.

Because the log is append-only and ordered, the full decision history of any run
is fixed once written and can be replayed deterministically.

## The decision cycle, end to end

Each trading cycle leaves a complete, ordered trail:

| Step | Event type | What it records |
| --- | --- | --- |
| Observe | `market_snapshot_received` | Number of assets / snapshot timestamp. |
| Classify | `regime_classified` | The market regime (`risk_on`/`risk_off`/`chop`/`breakout`). |
| Target | `portfolio_target_computed` | Headline + intended order count. |
| Score | `asset_scored` | Per-asset alpha score. |
| Propose | `order_proposed` | `from → to`, amount in USD. |
| Quote | `twak_quote_received` | Route + slippage. |
| **Rule** | `risk_approved` / `risk_clipped` / `risk_rejected` | The risk engine's verdict (with reasons on reject). |
| Execute | `twak_swap_submitted`, `tx_confirmed` | Submitted + confirmed on-chain trade. |
| Reconcile | `portfolio_reconciled` | Resulting NAV + position count. |
| Publish | `agent_report_published` | Run summary. |

The crucial property: **the risk verdict is logged separately from the
proposal.** A `risk_rejected` event records exactly why a trade did *not*
happen (e.g. *"stable reserve 8.6% below required 10%"*), so safety decisions are
as auditable as executed ones.

## Reconstructing the narrative

The decision journal turns the raw log back into a human-readable story —
without adding any information the log does not already contain:

```bash
python3 python-lab/analyze.py journal
# write it out:
python3 python-lab/analyze.py journal --out data/journal.md
```

`python-lab/guardrail_lab/journal.py` segments the event stream into cycles at
each `regime_classified` event and, for each cycle, narrates: the regime, the
top scored assets, the proposed orders, the risk engine's verdict (with
rejection reasons), the confirmed trades, and the reconciled NAV. The renderer
is a pure function of the log, so the journal is a *faithful projection* of the
record, not an independent claim.

It degrades gracefully: an empty log produces a clear "no data" note (exit 0),
never an error.

## Why this is verifiable autonomy

1. **No trust in narration.** The journal is derived from the log; anyone can
   re-run the deterministic renderer over the same `events` table and get the
   same narrative.
2. **Negative space is recorded.** Rejected and clipped trades are first-class
   events, so the safety system's interventions are auditable, not invisible.
3. **Independent gate is on the record.** Every `risk_approved` / `risk_clipped`
   / `risk_rejected` event documents that an independent risk authority — not
   the strategy — made the final execution decision (see
   `docs/ENSEMBLE.md` for how that gate sits below the strategy layer).
4. **Reproducible offline.** The log plus the committed strategy specs and
   `skills/ensemble.json` are enough to re-derive both the proposed allocations
   (see the `ensemble` subcommand) and the executed decisions (the `journal`
   subcommand) from the repository alone.

Together these mean the agent's autonomy is **inspectable after the fact**:
every action, and every refusal to act, is reconstructable from an append-only
record.
