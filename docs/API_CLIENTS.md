# Guardrail API Clients

The Guardrail Alpha API is a **read-only HTTP API**: all 68 routes are
side-effect-free `GET`s that never mutate the live book or the event log (see
[docs/api/openapi.yaml](api/openapi.yaml)). Because the surface is simple and
read-only, there are many equally valid ways to consume it. This page is the
single index of every client option in the repo, with a chooser table to pick
the right one.

Everything here is **offline-safe**: no real network, keys, or wallet are
required to run or verify any client. The API defaults to
`http://localhost:8080`; start it with `cargo run -p guardrail-api`.

## Chooser table

| Client            | Language / Runtime       | Location                                  | Best for                                                                 |
| ----------------- | ------------------------ | ----------------------------------------- | ----------------------------------------------------------------------- |
| TypeScript SDK    | Node 18+ / browser       | [`clients/typescript`](../clients/typescript) | App and dashboard integration; typed methods over `fetch`.              |
| Python SDK        | Python 3 (stdlib only)   | [`clients/python`](../clients/python)         | Scripts, notebooks, data work; zero install, no dependencies.           |
| Go SDK            | Go (stdlib only)         | [`clients/go`](../clients/go)                 | Go services and CLIs; idiomatic `context`-aware client.                 |
| MCP server        | Python 3 (stdlib only)   | [`clients/mcp`](../clients/mcp)               | Letting *other agents* consume Guardrail as MCP tools/resources/prompts.|
| LangChain tools   | Python 3 (+ optional LC) | [`clients/langchain`](../clients/langchain)   | Wiring Guardrail endpoints into a LangChain agent as tools.             |
| Web-lite cockpit  | Browser (single file)    | [`clients/web-lite`](../clients/web-lite)     | Drop-anywhere, embeddable live status page; no build step.              |
| Proof verifier    | Python 3 (stdlib only)   | [`clients/proof-verifier`](../clients/proof-verifier) | Independently verifying the agent's on-chain identity proof offline.    |
| Postman + .http   | Postman / VS Code / JetBrains | [`clients/postman`](../clients/postman)  | Manual exploration; clicking through every route by hand.               |
| Runnable examples | Node 18+ / Python 3      | [`clients/examples`](../clients/examples)     | Copy-paste starting points; end-to-end guided flows.                    |

### Quick decision guide

- **I'm writing application code** -> use the SDK for your language
  (TypeScript / Python / Go). They share the same route set and JSON shapes.
- **I want another AI agent to use Guardrail** -> the
  [MCP server](../clients/mcp) (tools/resources/prompts over stdio) or the
  [LangChain tools](../clients/langchain).
- **I just want to look at the data** -> the
  [web-lite cockpit](../clients/web-lite) (open the HTML) or the
  [Postman / `.http` collection](../clients/postman) (click Send).
- **I need to trust-but-verify the agent's identity** -> the
  [proof verifier](../clients/proof-verifier) re-derives every hash offline.
- **I want a working example to crib from** -> the
  [examples directory](../clients/examples).

## SDKs

All three SDKs are dependency-free, mirror the same route set, and treat the API
as read-only.

- **TypeScript** ([`clients/typescript`](../clients/typescript)) — typed
  `GuardrailClient` over the global `fetch`. Builds with `tsc`; usable from Node
  18+ and modern browsers.
- **Python** ([`clients/python`](../clients/python)) — stdlib-only
  `guardrail_client` package; no install required (add the directory to
  `sys.path` or `pip install -e .`).
- **Go** ([`clients/go`](../clients/go)) — idiomatic, `context`-aware client
  using only `net/http`, `encoding/json`, and `context`.

## Agent integrations

- **MCP server** ([`clients/mcp`](../clients/mcp)) — exposes the read-only API
  as MCP tools, resources, and prompts over JSON-RPC 2.0 on stdio. Reuses the
  Python SDK; no extra dependencies. Lets CMC Agent Hub / TWAK-style agents
  consume Guardrail with no glue of their own.
- **LangChain tools** ([`clients/langchain`](../clients/langchain)) — exposes
  Guardrail's research and status endpoints as LangChain agent tools. Works with
  or without `langchain_core` installed.

## Browser

- **Web-lite cockpit** ([`clients/web-lite`](../clients/web-lite)) — a
  zero-dependency, single-file `index.html` mission-control view. No build step,
  no CDNs; calls the API directly from the browser. Complementary to the full
  Next.js `dashboard/`.

## Verification

- **Proof verifier** ([`clients/proof-verifier`](../clients/proof-verifier)) —
  an independent, stdlib-only tool that re-derives every hash in the agent's
  identity proof (policy hash, run-report hash, deterministic `agent_id`) offline,
  without trusting the agent and without any network or chain access.

## Manual exploration: Postman + REST Client

[`clients/postman`](../clients/postman) contains request collections generated
directly from [docs/api/openapi.yaml](api/openapi.yaml) — one `GET` per route,
grouped into six folders (`readiness/competition`, `market data`,
`strategy/backtest`, `risk/portfolio`, `identity/proof`, `ops`):

- `guardrail.postman_collection.json` — Postman v2.1 collection with a
  `{{baseUrl}}` variable (default `http://localhost:8080`).
- `guardrail.http` — VS Code REST Client / JetBrains HTTP Client file with the
  same requests.

See [`clients/postman/README.md`](../clients/postman/README.md) for import and
usage instructions.

## Runnable examples

[`clients/examples`](../clients/examples) holds copy-paste-ready starting points.
All exit `0` even when the API is down, so they are safe to run any time.

| Example                  | Runtime | Shows                                                                 |
| ------------------------ | ------- | -------------------------------------------------------------------- |
| `python_quickstart.py`   | Python  | Guided 6-step SDK flow (health, policy compile, backtest, ...).      |
| `node_quickstart.mjs`    | Node 18+| Same 6-step flow against the TypeScript SDK surface via `fetch`.     |
| `ensemble_demo.py`       | Python  | Calls `/regime`, then prints the regime-routed ensemble blend (mirrors the cockpit). |
| `journal_export.mjs`     | Node 18+| Fetches `/events` and prints a compact decision journal.            |

```bash
# from the repo root
python3 clients/examples/python_quickstart.py
node    clients/examples/node_quickstart.mjs
python3 clients/examples/ensemble_demo.py
node    clients/examples/journal_export.mjs
```

Point any example or client at a non-default host with the
`GUARDRAIL_BASE_URL` environment variable.

## See also

- [docs/api/openapi.yaml](api/openapi.yaml) — the authoritative 68-route spec.
- [docs/ENSEMBLE.md](ENSEMBLE.md) — the regime-routed ensemble that
  `ensemble_demo.py` mirrors.
- [docs/EXPLAINABILITY.md](EXPLAINABILITY.md) — the event log that
  `journal_export.mjs` exports as a decision journal.
- [clients/examples/README.md](../clients/examples/README.md) — deeper notes on
  the quickstart examples.
