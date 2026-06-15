# What's New — Latest Additions

A concise changelog of the surfaces shipped most recently, grouped by prize lane.
Every entry cites the real file path(s) so a judge can read the source of truth
directly. Everything below is **offline-safe** — no API keys or chain access are
required to run or verify it.

Companion docs: [PRIZE_MAP.md](PRIZE_MAP.md) · [JUDGE_DEMO.md](JUDGE_DEMO.md) ·
[PITCH.md](PITCH.md) · [HACKATHON.md](HACKATHON.md) · [CLI.md](CLI.md) ·
[API.md](API.md) · [VERCEL_DEPLOY.md](VERCEL_DEPLOY.md).

---

## Phase 2 — engine, data, dashboard & tooling (newest)

- **Snapshot persistence (Track G).** The agent now persists periodic market
  snapshots to an append-only store under `data/snapshots/` so the analytics have
  real history to chart without replaying the full event log. A new read-only
  `GET /snapshots` route projects that history into a compact, chartable summary:
  retained count, time span, the latest snapshot's regime and per-asset prices, and
  a lightweight series (with an optional `?limit=` cap). The route only reads what
  the agent has already persisted — it never ingests, mutates, or backfills.
  Spec: [api/openapi.yaml](api/openapi.yaml) (path `/snapshots`,
  `SnapshotsResponse` schema). Plan: [`../PLAN_V2.md`](../PLAN_V2.md) Track G.
  Verify: `curl -fsS http://127.0.0.1:8080/snapshots`.

- **Live ensemble routing in the engine (Track C).** The regime-routed ensemble is
  promoted from the Python/cockpit advisory view into the **live Rust engine**:
  it blends the per-skill target books by classified regime confidence and defers
  to the `risk-engine` as the sole execution gate. Wired into the agent runtime
  behind a config flag; the existing `GET /ensemble` route surfaces the live
  blended book and per-skill attribution. Detail: [ENSEMBLE.md](ENSEMBLE.md).
  Plan: [`../PLAN_V2.md`](../PLAN_V2.md) Track C.

- **Dashboard lab / ensemble / journal pages.** The Next.js cockpit
  (`dashboard/`) gains three headline pages: **`/lab`** — a server-backed Strategy
  Lab that tunes inputs and re-runs the real backtest pipeline via `GET /backtest`
  (interactive metrics + equity-curve sparkline); **`/ensemble`** — the
  regime-routed meta-allocator weights, blended book, and per-skill attribution;
  and **`/journal`** — the per-cycle decision narrative (regime → scores → target
  → risk → execute) off the append-only event log. Full reference:
  [DASHBOARD.md](DASHBOARD.md).
  Files: `dashboard/src/app/{lab,ensemble,journal}/page.tsx`.

- **In-browser WASM backtesting deferred (honest note).** The PLAN_V2 Track B goal
  of running the real engine client-side via WASM is deferred: the compute stack
  (`backtester` → `market-data` → `cmc-client` → `reqwest`/`tokio`) is not
  `wasm32`-compatible. The `/lab` page provides the same capability **server-side**
  instead — same engine that gates live trades, no WASM build. Rationale and the
  path to revisit: [WASM_NOTE.md](WASM_NOTE.md).

- **Go `guardrailctl watch`.** A new Go operator CLI, `guardrailctl`, adds a
  `watch` subcommand that tails the read-only API (e.g. the SSE `/stream` and
  status routes) and prints a live, terminal-friendly status stream. It is a
  read-only consumer — it never signs or mutates — and is offline-safe against the
  paper-mode API.
  Verify: `guardrailctl watch` (against a running `guardrail-api`).

---

## Platform & developer experience (latest)

- **CLI modularization.** The `guardrail-cli` binary now keeps argument parsing
  and dispatch in `main.rs` and delegates every `run_*` implementation to a
  domain-grouped `commands/` tree: `backtest`, `market`, `portfolio`, `identity`,
  `reporting`, `experiment`, `agent_surface`, and `commerce`. 40 top-level
  subcommands (plus nested `policy` and `experiment` subcommands) are wired
  through `commands/mod.rs`.
  Files: `apps/guardrail-cli/src/commands/` (8 modules) + `apps/guardrail-cli/src/main.rs`.
  Full reference: [CLI.md](CLI.md).
  Verify: `ls apps/guardrail-cli/src/commands/`; `cargo run -p guardrail-cli -- --help`.

- **TUI live panels.** The terminal cockpit renders four live, data-driven panels
  off the event log and the latest run report: **regime** (classification +
  exposure), **positions** (book summary), **risk** (latest decisions / gate
  state), and **alerts** (staleness, drawdown, kill-switch). Each panel is its own
  module composed by `render.rs`.
  Files: `apps/guardrail-tui/src/{regime,positions,risk,alerts,render}.rs`.
  Verify: `cargo run -p guardrail-tui`.

- **Four new read-only API routes.** `apps/guardrail-api` now serves:
  - `GET /journal` — decision-journal projection of the agent's per-cycle
    reasoning chain (`apps/guardrail-api/src/journal.rs`).
  - `GET /ensemble` — the regime-routed ensemble meta-allocator configuration and
    current regime weights (`apps/guardrail-api/src/ensemble.rs`).
  - `GET /skills` — typed catalog projection of `skills/INDEX.json`
    (`apps/guardrail-api/src/skills.rs`).
  - `GET /version` — service version, build target, run mode, and uptime
    (`apps/guardrail-api/src/version.rs`).
  These bring the read-only surface to **68 routes** (`apps/guardrail-api/src/server.rs`).
  Spec: [api/openapi.yaml](api/openapi.yaml). Index: [API.md](API.md).
  Verify: `curl -fsS http://127.0.0.1:8080/version`; `curl -fsS http://127.0.0.1:8080/skills`.

- **Phase-2 expansion plan (PLAN_V2).** The next-phase roadmap — real-time engine
  telemetry (SSE), in-browser WASM backtesting, a native Rust strategy ensemble,
  a strategy marketplace + dynamic skill loader, a proof explorer + verifier UI,
  control/growth services, and the data pipeline — all offline-safe and
  conflict-safe, mapped to the four prize lanes.
  File: [`../PLAN_V2.md`](../PLAN_V2.md).

- **Vercel deployment.** The Next.js dashboard (`dashboard/`, 56 page routes)
  auto-deploys to Vercel on every push to `main`; pull requests get preview
  deployments. Root directory `dashboard`, `NEXT_PUBLIC_API_URL` points at a
  running `guardrail-api`. The dashboard is read-only and never holds keys.
  Files: `dashboard/vercel.json`. Guide: [VERCEL_DEPLOY.md](VERCEL_DEPLOY.md).

---

## Track 2 — Strategy Skills ($6k)

- **Two new strategy skills (now four total).** In addition to the general
  `regime-routed-bsc-alpha` and `funding-rate-carry-bsc`, the catalog now ships:
  - `mean-reversion-chop-bsc` — a range-fade specialist (RSI(14) + Bollinger(20,2)
    %B + ATR(14) stops) that peaks in the CHOP regime.
    Files: `skills/mean-reversion-chop/` (`skill.yaml`, `strategy_spec.yaml`,
    `SKILL.md`, `prompts/`, `examples/`, `tests/`).
  - `trend-breakout-momentum-bsc` — a momentum/breakout specialist (EMA(12/26/50)
    stack + MACD + Donchian(20) + volume confirmation) that peaks in the BREAKOUT
    regime. Files: `skills/trend-breakout-momentum/` (same layout).
  - Catalog index: `skills/INDEX.json` (all four skills enumerated).

- **Regime ensemble meta-allocator.** Blends the four skills' example target books
  by classified regime (weighted average → renormalize → USDT reserve), advisory
  only — the Rust risk engine remains the sole execution gate.
  Files: config `skills/ensemble.json`; blender `python-lab/guardrail_lab/ensemble.py`;
  CLI `python-lab/analyze.py` (`ensemble` subcommand). Detail: [ENSEMBLE.md](ENSEMBLE.md).
  Verify: `python3 python-lab/analyze.py ensemble --regime chop`.

- **Skill authoring kit.** A reproducible scaffold-and-lint workflow so judges (or a
  host LLM) can add a fifth skill in seconds.
  Files: template `skills/_template/`; scaffolder `scripts/new_skill.sh`;
  example validator `scripts/lint_skills.sh` (runs `guardrail_lab.skill` over each
  `skills/*/examples/`). Detail: [SKILL_AUTHORING.md](SKILL_AUTHORING.md).
  Verify: `bash scripts/new_skill.sh demo-skill && bash scripts/lint_skills.sh`.

- **Decision journal.** Renders the append-only event log as a human-readable,
  per-cycle decision narrative (regime → scores → target → risk → execute).
  Files: `python-lab/analyze.py` (`journal` subcommand).
  Verify: `python3 python-lab/analyze.py journal`.

## CMC — Best Use of Agent Hub ($2k)

- **MCP server now exposes tools + resources + prompts.** The server advertises the
  full Model Context Protocol capability surface (`capabilities: {tools, resources,
  prompts}`), making it Hub-ready rather than tools-only: 14 read-only tools, 5
  resources, and 3 prompts.
  Files: `clients/mcp/manifest.json`, `clients/mcp/run.py`, `clients/mcp/mcp.json`,
  `clients/mcp/guardrail_mcp/`; Rust transport `crates/cmc-client/src/mcp.rs`.
  Verify: `cat clients/mcp/manifest.json`.

- **Hub-ready manifest.** A single descriptor a host reads to register the server:
  protocol/transport, runtime command, env, and the tool/resource/prompt catalog.
  File: `clients/mcp/manifest.json`.

## BNB — Best Use of BNB AI Agent SDK ($2k)

- **Independent on-chain proof verifier.** A stdlib-only, clean-room Python tool that
  re-derives the agent's `policy_hash`, `report_hash`, `agent_id`, `address_url`,
  and the competition contract / tx URL formats from first principles and compares
  them to the claimed proof — sharing no code with the Rust agent ("don't trust,
  verify").
  Files: `clients/proof-verifier/verify.py`, `clients/proof-verifier/sample_proof.json`,
  `clients/proof-verifier/README.md`; wrapper `scripts/verify_proof.sh`.
  Detail: [PROOF_VERIFICATION.md](PROOF_VERIFICATION.md).
  Verify: `bash scripts/verify_proof.sh` (auto-selects the run report or the bundled
  offline fixture).

## TWAK — Best Use of TWAK ($2k)

- **Self-custody demo.** A narrated, fully offline walkthrough of the
  agent-proposes → risk-gates → TWAK-signs-with-user-keys → execute/reconcile flow.
  Never loads or requires any key material; points at the real enforcing files and
  HTTP routes.
  Files: `scripts/self_custody_demo.sh`. Detail:
  [TWAK_SELF_CUSTODY_DEMO.md](TWAK_SELF_CUSTODY_DEMO.md) · [SELF_CUSTODY.md](SELF_CUSTODY.md).
  Verify: `bash scripts/self_custody_demo.sh`.

- **Example signing policy.** An illustrative TWAK authorization envelope documenting
  per-tx / daily / session caps, allowed and forbidden actions, the allowed
  contracts and assets, and the x402 `primaryType` allow/deny-list. Keys never leave
  the user's wallet; TWAK is the sole signer.
  File: `configs/signing_policy.example.json` (served at `GET /signing-policy`).
  Verify: `cat configs/signing_policy.example.json`.

## Cross-cutting — operability & demo surfaces

- **Web-lite cockpit tabs.** The zero-build single-file cockpit gained **Ensemble**
  (per-skill regime blend, mirroring `skills/ensemble.json`), **Journal**
  (decision narrative off `/events`), and **Signing** (the self-custody envelope off
  `/signing-policy`) tabs.
  File: `clients/web-lite/index.html`.

- **Scenario library.** A set of deterministic stress scenarios (flash crash, funding
  spike, kill-switch trip, liquidity crunch, market stress, regime whipsaw) for
  exercising the risk controls offline.
  Files: `configs/scenarios/` (`index.json` + per-scenario JSON); served at
  `GET /scenarios` (`apps/guardrail-api/src/scenarios.rs`).
  Verify: `curl -fsS http://127.0.0.1:8080/scenarios`.

- **Alert relay.** The watchdog now relays alerts through a reusable notifier crate
  with Console, File, and outbound Webhook sinks (the webhook sink fires when
  `GUARDRAIL_WEBHOOK` is configured; offline runs use the console sink).
  Files: `crates/notifier/src/lib.rs`; wiring `apps/guardrail-monitor/src/notify.rs`.
  Verify: `GUARDRAIL_MONITOR_CHECKS=1 cargo run -p guardrail-monitor`.
