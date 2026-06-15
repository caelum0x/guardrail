# ADR 0001 — Rust-native live engine

- Status: Accepted
- Date: 2026-06-13

## Context

The agent trades live capital on BSC: it must be correct, memory-safe, and
predictable under a continuous autonomous loop. We also want fast research
iteration (charts, analysis) and a clean operator UI.

## Decision

Split by responsibility and pick the right tool for each:

- **Rust** owns the entire live path — market data, features, strategy, risk,
  execution, portfolio accounting, event store, runtime. A Cargo workspace with
  small single-responsibility crates.
- **Python** (`python-lab/`) is reserved for research and analytics only. It
  reads the event log / run report and produces charts and reports. It never
  trades, holds keys, or runs the decision loop.
- **TypeScript** (`dashboard/`) is a read-only Next.js cockpit. It renders API
  data and never reaches the executor.

## Consequences

- Money math uses `rust_decimal::Decimal` end to end (no float drift).
- The decision/execution path has no GC pauses and strong compile-time guarantees.
- Cross-language coupling is one-directional and through stable artifacts (SQLite
  event log, `run_report.json`, the read-only HTTP API) — never shared mutable state.
- Analytics/UI can evolve independently without risk to the trading engine.
