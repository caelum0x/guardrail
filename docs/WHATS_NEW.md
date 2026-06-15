# What's New — Latest Additions

A concise changelog of the surfaces shipped most recently, grouped by prize lane.
Every entry cites the real file path(s) so a judge can read the source of truth
directly. Everything below is **offline-safe** — no API keys or chain access are
required to run or verify it.

Companion docs: [PRIZE_MAP.md](PRIZE_MAP.md) · [JUDGE_DEMO.md](JUDGE_DEMO.md) ·
[PITCH.md](PITCH.md) · [HACKATHON.md](HACKATHON.md).

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
