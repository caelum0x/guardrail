# ADR 0004 — Append-only SQLite event log as the book of record

- Status: Accepted
- Date: 2026-06-13

## Context

We need a tamper-evident, replayable record of every decision for debugging,
the dashboard, and judge-facing proof — without standing up external infra.

## Decision

The agent writes an append-only event stream to a local SQLite database
(`data/guardrail_alpha.db`) via `event-store::SqliteEventRepository`, plus a
rolling `data/run_report.json` snapshot. `AgentEvent` captures the full lifecycle:
`AgentStarted, MarketSnapshotReceived, RegimeClassified, AssetScored,
PortfolioTargetComputed, OrderProposed, RiskApproved/Rejected/Clipped,
TwakQuoteReceived, TwakSwapSubmitted, TxConfirmed, PortfolioReconciled,
DrawdownThrottleActivated, KillSwitchTriggered, DailyTradeRequirementSatisfied,
AgentReportPublished`.

Everything else reads this log: the API (`/events`, `/cockpit`, `/history`,
`/risk`, `/proof`), the exporter (Prometheus metrics), `guardrail-replay`
(audit), and the python analytics.

## Consequences

- Bundled SQLite (rusqlite) means zero external dependencies; the DB is a single
  file shareable across the co-located read-only sidecars (see deploy/k8s).
- The log answers "why did it trade, what was quoted, what tx resulted, what
  changed after" purely from history — no live state required.
- Each run is fingerprinted: `policy_hash` + `report_hash` in the published
  report, anchorable on-chain via `bnb-agent` proof.
- Trade-off: one writer (the agent). Reads are concurrent; the agent is the sole
  writer, which the deployment topology enforces.
