# WASM Note — In-browser real-engine backtesting is deferred

This is an honest record of a scope decision about [PLAN_V2.md](../PLAN_V2.md)
**Track B — In-browser backtesting (WASM)**.

## What Track B proposed

Track B proposed compiling the *real* Rust strategy + risk + backtester engine to
`wasm32-unknown-unknown` via `wasm-bindgen`/`wasm-pack` (a `crates/strategy-wasm/`
wrapper exposing `run_backtest(mandate, preset)`), so the `/lab` page could run the
exact same risk engine that gates live trades **entirely client-side**, with no
server round-trip.

## Why it is deferred

The engine's compute stack is **not `wasm32`-compatible** as it stands. The
backtest path pulls in a dependency chain that does not compile to (or run
meaningfully on) `wasm32-unknown-unknown` without a substantial, risky rewrite:

```
backtester → market-data → cmc-client → reqwest / tokio
```

- `cmc-client` uses `reqwest`, which depends on a native TLS/HTTP transport that
  is not available on `wasm32-unknown-unknown`.
- `tokio`'s multi-threaded runtime, timers, and I/O are not available in that
  target either.
- Porting would mean splitting the pure-compute core away from the I/O layers and
  introducing a parallel `wasm`-only transport — net-new surface area with no
  offline-safety or correctness benefit for the demo, and a real risk of the
  WASM build and the native engine drifting apart.

Forcing the engine into the browser would therefore trade a guarantee we already
have (the dashboard runs the **same** engine the agent runs) for a parallel,
harder-to-verify code path. That is the wrong trade for this phase.

## What we shipped instead

The **`/lab` page provides the same capability server-side.** It calls the
read-only `GET /backtest` route, which runs the real strategy + risk + backtester
pipeline in-process in `guardrail-api`, and renders the interactive controls,
metrics, and equity-curve sparkline in the browser.

This keeps the most important property intact: **the backtest the judge runs in
the cockpit is the same engine that gates live trades** — just executed on the
server rather than in WASM. It stays fully offline-safe (paper mode, deterministic
mocks, no keys, no chain access).

See [DASHBOARD.md](DASHBOARD.md#lab--server-backed-strategy-lab) for the `/lab`
page and [api/openapi.yaml](api/openapi.yaml) for the `/backtest` contract.

## If Track B is revisited

The path forward is to extract a pure-compute core crate (strategy + risk +
backtester math, no `reqwest`/`tokio`, snapshots passed in as data) and compile
*only that* to `wasm32`, feeding it snapshots fetched by the browser from the
read-only API. Until that refactor is justified, the server-backed `/lab` page is
the supported in-cockpit backtesting surface.
