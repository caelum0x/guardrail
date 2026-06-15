# Guardrail SDK Examples

Runnable end-to-end quickstarts for both Guardrail client SDKs. Each one runs
the same guided sequence against the read-only Guardrail Alpha API and prints a
concise summary of each call.

## Prerequisites

Start the API first (from the repo root):

```bash
cargo run -p guardrail-api
```

By default it listens on `http://localhost:8080`. Point the examples elsewhere
with the `GUARDRAIL_BASE_URL` environment variable.

Both examples degrade gracefully: if the API is unreachable they print a
friendly notice and exit `0` (never a stack trace), so they are safe to run
even before the API is up.

## Python (`python_quickstart.py`)

Stdlib-only. Reuses the published SDK at `../python/guardrail_client` by adding
that directory to `sys.path` (no install required).

```bash
# from the repo root
python3 clients/examples/python_quickstart.py

# against a custom host
GUARDRAIL_BASE_URL=http://127.0.0.1:9000 python3 clients/examples/python_quickstart.py
```

## Node (`node_quickstart.mjs`)

Dependency-free. Requires Node 18+ (uses the global `fetch`). It mirrors the
TypeScript SDK's method set and routes (`../typescript/src/index.ts`) using
`fetch` directly, so it runs without a `tsc` build step.

```bash
# from the repo root
node clients/examples/node_quickstart.mjs

# against a custom host
GUARDRAIL_BASE_URL=http://127.0.0.1:9000 node clients/examples/node_quickstart.mjs
```

## What they demonstrate

Both scripts walk the same six-step flow, exercising the core SDK surface:

| Step | SDK call                                              | Route             | Shows                                  |
| ---- | ---------------------------------------------------- | ----------------- | -------------------------------------- |
| 1    | `health()`                                           | `/health`         | API + database status                  |
| 2    | `compile_policy(...)` / `compilePolicy(...)`         | `/policy/compile` | NL mandate -> validated policy hash    |
| 3    | `backtest(steps=60, fear_greed=70, preset=balanced)` | `/backtest`       | strategy vs benchmark, returns, drawdown |
| 4    | `walkforward()`                                      | `/walkforward`    | rolling out-of-sample windows          |
| 5    | `regime()`                                           | `/regime`         | current market regime                  |
| 6    | `compete()`                                          | `/compete`        | competition status                     |

The compiled mandate used in step 2 is:

> `Trade CAKE max drawdown 20% kill switch 25%`
