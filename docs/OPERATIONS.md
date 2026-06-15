# Operations Runbook

A single operator guide that ties the Guardrail tooling together: how to run the
stack, watch alerts, run analytics, exercise the stress scenarios, and verify
the agent's identity. **Everything here is offline-safe** — it runs in paper
mode with deterministic mocks and needs no API keys or network access.

All commands are written to be run from the repository root. The architectural
"why" behind these tools lives in the ADRs (`docs/adr/`) and the topic docs
(`docs/ENSEMBLE.md`, `docs/EXPLAINABILITY.md`, `docs/SKILL_AUTHORING.md`,
`docs/ALERTING.md`, `docs/SCENARIOS.md`).

---

## 1. Run the stack

The one-command entry point for evaluators is the judge quickstart. It builds
the workspace, runs the paper agent once to populate `data/` if no run data
exists, starts `guardrail-api` on `:8080` (waiting for `/health`), and serves
the web-lite cockpit wired to the API.

```bash
# Build + run paper agent + serve API and cockpit (offline, deterministic).
scripts/judge_quickstart.sh

# Skip the cargo build (reuse an existing target/).
scripts/judge_quickstart.sh --no-build

# Override ports.
scripts/judge_quickstart.sh --port 8095 --api-port 8080

# Release build instead of debug.
GUARDRAIL_RELEASE=1 scripts/judge_quickstart.sh
```

Background processes are cleaned up on exit. Once it prints its URL panel, the
API is reachable for the read-only endpoints used by the rest of this runbook
(`/health`, `/alerts`, `/readiness`, `/events`, `/proof`, `/scenarios`).

---

## 2. Watch alerts

The alert relay is an **out-of-process, read-only consumer** of the API
(ADR 0009): it polls `GET /alerts`, dedups, filters by severity, and forwards to
chat sinks. It is `--dry-run` by default and never crashes on a down API.

```bash
# Offline smoke test: single poll, dry-run (DEFAULT). Safe even with the API down.
python3 integrations/alert-relay/relay.py --once --dry-run

# Continuous dry-run loop (prints what WOULD be sent).
python3 integrations/alert-relay/relay.py

# Live single poll (real delivery; needs secrets in env + enabled sinks).
python3 integrations/alert-relay/relay.py --once --live

# Use a custom config file.
python3 integrations/alert-relay/relay.py --config configs/alerts.json
```

Delivery secrets (Telegram token, Discord/webhook URLs) come from environment
variables at runtime, never from code. See `integrations/alert-relay/README.md`
for the config schema and `docs/ALERTING.md` for severity routing.

---

## 3. Run analytics

All analytics read the append-only event log (`data/guardrail_alpha.db`) and the
run report (`data/run_report.json`) — the same source of truth the agent writes
(ADR 0004, ADR 0007). Every subcommand is offline and deterministic.

```bash
# Regime transition / time / exposure analytics.
python3 python-lab/analyze.py regime

# NAV underwater curve and worst drawdown episodes.
python3 python-lab/analyze.py drawdown --top-n 5

# IID bootstrap risk simulation over the NAV curve (VaR/CVaR).
python3 python-lab/analyze.py montecarlo --paths 2000 --seed 7

# Synthesize every analytic into one Markdown research dossier.
python3 python-lab/analyze.py dossier --out data/dossier.md

# Blend the 4 Track-2 skills by regime (book + per-skill attribution).
python3 python-lab/analyze.py ensemble --regime breakout

# Compare the blended ensemble book vs. each single skill.
python3 python-lab/analyze.py ensemble-compare --all

# Human-readable per-cycle decision journal from the event log.
python3 python-lab/analyze.py journal --out data/journal.md
```

The `ensemble` / `ensemble-compare` subcommands re-derive the meta-allocator's
book purely from `skills/ensemble.json` and the committed skill examples
(ADR 0006). The `journal` subcommand is a pure projection of the event log
(ADR 0007). To validate or scaffold strategy skills (ADR 0008):

```bash
bash scripts/new_skill.sh my-new-strategy-bsc   # scaffold from skills/_template
bash scripts/lint_skills.sh                      # run the real validator on all skills
```

---

## 4. Exercise the stress scenarios

The scenario library walks each named stress config in `configs/scenarios/` and
shows the exact guardrail response (throttle / reduce-only / kill switch /
stop-loss) it is expected to trigger (see `docs/SCENARIOS.md`).

```bash
bash scripts/run_scenarios.sh
```

This is fully offline; it drives `guardrail-sim` against
`configs/risk_policy.production.json`. The live `GET /scenarios` endpoint on the
running API gives the complementary pre-trade desk view.

---

## 5. Verify identity / proof

Independently re-derive the agent's `policy_hash`, `report_hash`, and `agent_id`
from first principles and check the competition contract + explorer URL formats.
No network or keys required.

```bash
# Verify the live run report if present, else the bundled offline fixture.
scripts/verify_proof.sh

# Verify an explicit proof document.
scripts/verify_proof.sh path/to/proof.json
```

Exits `0` only when every applicable check passes. The verifier itself
(`clients/proof-verifier/verify.py`) is standard-library-only. See
`docs/PROOF_VERIFICATION.md` for what each check asserts.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `scripts/judge_quickstart.sh` hangs waiting for `/health` | API port already in use | Re-run with a different `--api-port`, or free `:8080`. |
| Quickstart shows no data in the cockpit | `data/` was empty and the paper run was skipped | Run `scripts/judge_quickstart.sh` without `--no-build` so the paper agent populates `data/`. |
| `analyze.py` prints "no data" / empty output | Event log `data/guardrail_alpha.db` not yet populated | Run the stack first (section 1) to generate a paper run. |
| `analyze.py ensemble` shows an empty book + a `reason` | Missing `skills/ensemble.json` or a skill example file | Restore the config / examples; confirm with `bash scripts/lint_skills.sh`. |
| `relay.py` logs `API unreachable / no alerts` | API not running | Start the stack (section 1); dry-run still exits 0 by design. |
| `relay.py --live` sends nothing | No sinks enabled or secrets missing in env | Enable sinks in the config and export the required tokens/URLs. |
| `lint_skills.sh` reports FAIL | An example over-allocates (>100) or is missing `symbol`/`weight_pct` | Fix the offending `examples/*.json`; leave the remainder as USDT reserve. |
| `verify_proof.sh` exits non-zero | Proof hashes/contract mismatch | Re-generate the run report via the stack, or pass a known-good proof path. |
| `command not found: python3` | Python not on PATH | Install Python 3.8+, or set `PYTHON_BIN` for the shell scripts. |
