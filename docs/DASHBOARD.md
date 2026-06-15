# Dashboard

The Guardrail Alpha dashboard (`dashboard/`) is a read-only Next.js (App Router)
cockpit over the read-only Guardrail API. It holds **no keys**, never signs, and
never mutates the live book or the event log — every page is a projection of a
`GET` endpoint served by `apps/guardrail-api`. It is offline-safe: with the API
running in paper mode against deterministic mocks, every page renders without API
keys or chain access.

The dashboard auto-deploys to Vercel on every push to `main` (preview deployments
on pull requests). Set `NEXT_PUBLIC_API_URL` to a reachable `guardrail-api`. See
[VERCEL_DEPLOY.md](VERCEL_DEPLOY.md) for the deployment specifics and
[API.md](API.md) / [api/openapi.yaml](api/openapi.yaml) for the backing routes.

## How it works

- **Server components fetch on the server.** Pages call the API through
  `dashboard/src/lib/api.ts` (`getJsonOrNull`) so a missing or unreachable
  endpoint degrades gracefully to an empty/placeholder render instead of crashing.
- **Navigation** is a flat link list in `dashboard/src/components/Layout.tsx`;
  every entry maps one-to-one to a page route under `dashboard/src/app/`.
- **Live updates** come from `AutoRefresh` (a 5s soft refresh) for polling pages
  and from `EventSource` against the SSE `/stream` route for the real-time `/live`
  page. See [REALTIME.md](REALTIME.md).

## Headline pages

These are the pages a judge or operator should see first. Each is backed by the
named read-only API route(s).

### `/` — Cockpit
The landing page: regime, book summary, latest risk decisions, and alerts at a
glance. Backed by `/portfolio`, `/risk`, `/alerts`, and the run report.

### `/live` — Real-time telemetry
A push-based cockpit that subscribes to the SSE `/stream` endpoint and updates
regime, trades, NAV, and risk decisions in real time as the agent logs each
`AgentEvent` — no polling. This is the "verifiable autonomy in real time" surface
for Track 1.
Page: `dashboard/src/app/live/page.tsx`. Route: `GET /stream`. Detail:
[REALTIME.md](REALTIME.md).

### `/lab` — Server-backed Strategy Lab
An interactive backtest workbench. Tune the strategy inputs (steps, fear/greed
sentiment, and a risk preset) and re-run the **real** Rust strategy + risk +
backtester pipeline, then read the metrics (total return, buy-and-hold benchmark,
excess/alpha, max drawdown, trade count, win rate, profit factor, final NAV) and a
self-contained SVG equity-curve sparkline.

The compute runs **server-side** against `GET /backtest` rather than in the
browser: the engine's compute stack (`backtester` → `market-data` → `cmc-client` →
`reqwest`/`tokio`) is not `wasm32`-compatible, so the in-browser-WASM track is
deferred and `/lab` delivers the same "tune-and-backtest in the cockpit"
capability without a WASM build. See [WASM_NOTE.md](WASM_NOTE.md) for the honest
record of that decision.
Page: `dashboard/src/app/lab/page.tsx`
(+ `components/LabControls.tsx`, `components/PresetSelect.tsx`). Route:
`GET /backtest?steps=&fear_greed=&preset=`.

### `/ensemble` — Regime-routed ensemble
Renders the regime-routed ensemble meta-allocator: the per-skill weights by
classified regime, the blended target book, and per-skill attribution. The
ensemble is advisory only — the Rust risk engine remains the sole execution gate.
Page: `dashboard/src/app/ensemble/page.tsx`. Route: `GET /ensemble`. Detail:
[ENSEMBLE.md](ENSEMBLE.md).

### `/journal` — Decision journal
The decision-journal projection of the append-only event log: the agent's
per-cycle reasoning chain (regime → scores → target → risk → execute) rendered as
a human-readable narrative. This is the explainability surface — reconstructing
*why* the agent acted from a tamper-evident record.
Page: `dashboard/src/app/journal/page.tsx`. Route: `GET /journal`. Detail:
[EXPLAINABILITY.md](EXPLAINABILITY.md).

### `/skills` — Strategy marketplace
The Track-2 strategy-skill catalog: cards for each registered skill with its
summary, the regimes it specializes in, and a "backtest this skill" action. Drill
into a skill at `/skills/[id]`, which is backed by `GET /skills/{id}` (detail) and
`GET /skills/{id}/backtest?preset=` (run the real pipeline contextualized by the
skill).
Pages: `dashboard/src/app/skills/page.tsx`, `dashboard/src/app/skills/[id]/page.tsx`.
Routes: `GET /skills`, `GET /skills/{id}`, `GET /skills/{id}/backtest`. Detail:
[SKILLS_MARKETPLACE.md](SKILLS_MARKETPLACE.md).

### `/proof` — Proof explorer + verifier
A proof explorer over the agent's BNB identity: the identity record, ERC-8004 /
8183 registry records, server-side hash-recomputation status, and BscScan /
BscTrace deep links. Backed by `GET /proof` and the independent verifier
`GET /proof/verify`, which recomputes the sha256 policy/report hashes and checks
the contract and tx URL formats — "identity is verifiable, not cosmetic."
Page: `dashboard/src/app/proof/page.tsx` (+ `components/ProofCard.tsx`). Routes:
`GET /proof`, `GET /proof/verify`. Detail:
[PROOF_VERIFICATION.md](PROOF_VERIFICATION.md).

## Supporting pages

Every other navigation entry is a focused projection of a single read-only route.
Grouped by purpose:

- **Portfolio & market data:** `/portfolio`, `/assets`, `/watchlist`,
  `/liquidity`, `/indicators`, `/trending`, `/quotes`, `/exposure`, `/equity`,
  `/trades`, `/signals`.
- **Risk & operations:** `/risk`, `/alerts`, `/heartbeat`, `/rebalance`, `/drift`,
  `/exit-triggers`, `/scenarios`, `/wallet-controls`, `/ops`, `/observability`,
  `/readiness`, `/events`.
- **Strategy & analytics:** `/regime`, `/funding`, `/backtest`, `/walkforward`,
  `/sweep`, `/research`, `/optimizer`, `/costs`, `/budget`, `/mandates`,
  `/experiments`, `/compile`, `/skill`.
- **Agent surface & identity:** `/bnb-sdk`, `/agent-card`, `/sdk-catalog`,
  `/agent-services`, `/job-simulator`, `/signing-policy`, `/commerce`, `/compete`.
- **Config & reporting:** `/policy`, `/universe`, `/config`, `/playbook`,
  `/briefing`, `/prizes`, `/scorecard`, `/audit-manifest`, `/reports`.

## Verifying locally

```bash
# 1. Start the read-only API in paper mode (offline, deterministic mocks).
cargo run -p guardrail-api

# 2. Point the dashboard at it and run the dev server.
cd dashboard && NEXT_PUBLIC_API_URL=http://127.0.0.1:8080 npm run dev

# 3. Open http://localhost:3000 and visit /live, /lab, /ensemble, /journal,
#    /skills, and /proof.
```

The dashboard is a strictly read-only consumer of the API. For the machine-readable
contract of every route it depends on, see [api/openapi.yaml](api/openapi.yaml).
