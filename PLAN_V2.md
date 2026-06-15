# Guardrail Alpha — Phase 2 Expansion Plan

> The original architecture in [`plan.md`](./plan.md) is fully realized (every
> crate, app, config, migration, doc, script, infra file, dashboard page, and
> skill it names exists; the build exceeds it). This document is the **next
> phase**: net-new folders, files, and functionality that deepen the product and
> the four hackathon prize lanes — Track 1 (autonomous trading), Track 2 (skills),
> CMC Agent Hub, BNB SDK, and TWAK self-custody.
>
> Everything stays **offline-safe** (paper/mock, no keys, no live token launches)
> and **conflict-safe** (new crates/dirs; existing shared files get one owner per
> change). Each increment must build green (`cargo build` + `clippy`,
> `tsc --noEmit`) before commit, then push to GitHub; the dashboard auto-deploys
> to Vercel on push. No CI suites and no dedicated test trees this phase
> (inline Rust unit tests only where natural).

---

## Track A — Real-time engine telemetry (live, not polling)

Today the dashboard/cockpit poll the read-only API every 5s. Make it **push**.

New:
- `crates/event-bus/` — a thin `tokio::sync::broadcast` fan-out over the
  append-only event store; the agent publishes each `AgentEvent` as it is logged.
- `apps/guardrail-api/src/stream.rs` — an SSE endpoint `GET /stream` (and
  `/stream/{topic}`) that serializes live events to the browser.
- `dashboard/src/lib/stream.ts` + a live `/live` cockpit page that subscribes to
  `/stream` and updates regime, trades, NAV, and risk decisions in real time.
- `clients/web-lite` — a `Live` tab using `EventSource`.

Prize fit: Track 1 (visible, verifiable autonomy in real time).

## Track B — In-browser backtesting (WASM)

Run the *real* Rust strategy + risk + backtester engine **client-side**.

New:
- `crates/strategy-wasm/` — `wasm-bindgen` wrapper exposing `run_backtest(mandate, preset)`
  over `strategy-engine` + `risk-engine` + `backtester` (compiled to `wasm32-unknown-unknown`).
- `dashboard/src/wasm/` (generated pkg) + a `/lab` page: type a natural-language
  mandate → compile policy → backtest in the browser → equity/drawdown charts, no server.
- `scripts/build_wasm.sh` — `wasm-pack build` into the dashboard.

Prize fit: differentiation; the same risk engine that gates live trades runs in the judge's browser.

> **Status — deferred (honest note).** The in-browser-WASM-real-engine track is
> deferred: the compute stack (`backtester` → `market-data` → `cmc-client` →
> `reqwest`/`tokio`) is not `wasm32`-compatible. The `/lab` page provides the same
> capability **server-side** instead, calling `GET /backtest` so the cockpit runs
> the exact same engine that gates live trades. Full rationale and the path to
> revisit it: [`docs/WASM_NOTE.md`](docs/WASM_NOTE.md).

## Track C — Native Rust strategy ensemble (live)

Promote the Python/cockpit ensemble into the **live engine** (the deferred Phase-1 native piece).

New:
- `crates/strategy-ensemble/` — loads `skills/ensemble.json` + the per-skill specs,
  blends target portfolios by regime confidence, defers to `risk-engine` as the sole gate.
- Wire into `crates/agent-runtime` behind a config flag (`strategy.mode = "ensemble"`).
- `apps/guardrail-api` `/ensemble/live` — the live blended book + per-skill attribution
  (the existing `/ensemble` route from batch-39 stays as the static view).
- `crates/backtester` — ensemble-vs-single comparison run.

Prize fit: Track 2 (ties the 6 skills into one live strategy).

## Track D — Strategy marketplace + dynamic skill loader

Make skills first-class, discoverable, and runnable end-to-end.

New:
- `crates/skill-loader/` — parse `skills/*/strategy_spec.yaml` at runtime into a typed
  registry; validate against the same contract `python-lab/guardrail_lab/skill.py` enforces.
- `apps/guardrail-api` — `/skills/{id}` (detail) and `/skills/{id}/backtest?preset=` (run).
- `dashboard/src/app/skills/` — a marketplace page (cards, regime routing, "backtest this skill").
- `clients/web-lite` — a richer Skills tab driven by the loader.

Prize fit: Track 2 (reusability + a real catalog/runner).

## Track E — Proof explorer + independent verifier UI

Surface the BNB identity/proof so judges can verify it themselves.

New:
- `apps/guardrail-api/src/proof_verify.rs` — `GET /proof/verify` recomputes the
  sha256 policy/report hashes and checks the contract + BscScan URLs server-side.
- `dashboard/src/app/proof/` — deepen into a proof explorer: identity, ERC-8004/8183
  records, hash recomputation status, BscScan/BscTrace deep links.
- Reuse `clients/proof-verifier` logic; optional WASM verifier widget.

Prize fit: BNB SDK ($2k) — "identity is verifiable, not cosmetic."

## Track F — Control & growth services

New top-level `services/` tree (out-of-process, read-only, offline-safe):
- `services/control-bot/` — a stdlib Telegram/Discord bot exposing **read-only**
  commands (`/status`, `/regime`, `/journal`, `/verify`) over the API; never signs.
- `services/gateway/` — a tiny edge (rate-limit + CORS + caching) fronting the API
  for public dashboard access.
- `services/report-publisher/` — renders the daily signed report bundle (reuses the
  python-lab report bundler) and writes it to a published `reports/` dir.

Prize fit: operational maturity; CMC Agent Hub (bot is another MCP/agent surface).

## Track G — Close the last plan.md gap + data pipeline

- `python-lab/notebooks/` — the 6 planned notebooks (`01_universe_filtering` …
  `06_submission_charts`) that load the event log and render the analyses (closes the
  one structural gap from `plan.md`).
- `crates/market-data` companion `data/snapshots/` ingestion: `apps/guardrail-agent`
  persists periodic market snapshots so the analytics have real history to chart.

---

## Build order & conflict-safety

1. **G (notebooks)** + **C (ensemble crate)** + **D (skill-loader crate)** — new
   files/crates, no collisions. New crates register via the workspace members glob.
2. **A (event-bus + SSE)** + **E (proof_verify)** — additive API modules + new crate.
3. **B (WASM)** + dashboard pages for C/D/E — dashboard is single-owner per batch.
4. **F (services/)** — entirely new top-level tree.

Each batch: disjoint dirs, one owner per shared file (`server.rs`, `Cargo.toml`,
dashboard layout), build green + clippy clean, then commit + push (Vercel
auto-deploys the dashboard). Parallelize across **different** crates/apps via
subagents; serialize anything that edits the same file.

## Definition of done (per track)
- Builds green (`cargo build` + `cargo clippy -D warnings`; `tsc --noEmit` for dashboard).
- Runs offline (mock/paper; no keys, no network required to demo).
- Pushed to `github.com/caelum0x/guardrail`; dashboard live on Vercel.
- Mapped in `docs/PRIZE_MAP.md` / `docs/WHATS_NEW.md`.
