# Architecture Decision Records

Short, dated records of load-bearing decisions. Each captures the context, the
decision, and the consequences so future changes know what they're trading away.

| ADR | Decision |
|---|---|
| [0001](./0001-rust-native-engine.md) | Rust-native live engine; Python for analytics; TS for the dashboard |
| [0002](./0002-risk-engine-is-the-only-gate.md) | The risk engine is the only path to execution |
| [0003](./0003-twak-only-execution.md) | All signing/execution goes through TWAK (self-custody) |
| [0004](./0004-sqlite-append-only-event-log.md) | Append-only SQLite event log as the book of record |
| [0005](./0005-deterministic-mocks-for-paper.md) | Deterministic mocks for paper/backtest reproducibility |

Status values: Proposed · Accepted · Superseded.
