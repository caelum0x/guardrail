# ADR 0005 — Deterministic mocks for paper & backtest

- Status: Accepted
- Date: 2026-06-13

## Context

Demos, CI, and research must be reproducible and runnable with no API keys or
network. But the same code must also run live against real CMC + TWAK.

## Decision

Both external boundaries are traits with a live impl and a deterministic mock:

- `cmc_client::CmcDataSource` → `CmcRestClient` (live) / `MockCmcClient`
  (symbol-seeded, advancing tick). The runtime picks the source from config:
  live when `cmc.use_mock = false` **and** `CMC_API_KEY` is set, else mock.
- `twak_client::TwakExecutor` → real transport (MCP/REST/CLI) / `MockTwakClient`
  (impact-based slippage, synthetic receipts).

The backtester reuses the *production* `StrategyEngine` + `RiskEngine` +
`PortfolioState` over a deterministic synthetic price path
(`backtester::synthetic`): a per-symbol oscillation plus a sentiment drift
derived from Fear & Greed, with an equal-weight buy-and-hold benchmark.

## Consequences

- `scripts/demo.sh`, the agent paper loop, backtest/walk-forward/sweep, and the
  e2e test all run offline and produce stable output.
- Backtests validate the *real* strategy/risk logic, not a parallel
  reimplementation — only data and fills are substituted.
- The synthetic path is explicitly a model (documented in
  `docs/BACKTEST_METHODOLOGY.md`), not historical data; results show the
  strategy's risk-managed shape (capital preservation in fear, lagging an all-in
  benchmark in euphoria), not a performance claim.
