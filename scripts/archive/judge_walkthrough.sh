#!/usr/bin/env bash
# ============================================================================
# Guardrail Alpha — GUIDED JUDGE WALKTHROUGH (offline-safe)
# ============================================================================
# A single narrated tour of the whole product that needs NO live network and
# NO API keys. It runs the real tools in order, with a banner + a short plain-
# English explanation before each step, and finishes with a "what you just saw"
# summary plus pointers to the cockpit and dashboard.
#
# Design contract:
#   * Every step is GUARDED. A missing tool or a non-zero step is reported and
#     the tour continues — the walkthrough never hard-fails as a whole and
#     always exits 0 so a judge can run it unattended.
#   * Reads only local data. If scripts/seed_demo_data.sh exists it is used to
#     (re)seed; otherwise the existing data/ is used and we say so.
#   * Analytics run against the demo DB when present, else analyze.py's default.
#
# Usage:
#   bash scripts/judge_walkthrough.sh
#
# Companion docs: examples/walkthrough/README.md (the ~5-minute narrative).
# ============================================================================
set -euo pipefail

# Resolve repo root from this script's location so it works from any cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"
cd "${REPO_ROOT}"

PYTHON_BIN="${PYTHON_BIN:-python3}"
ANALYZE_PY="${REPO_ROOT}/python-lab/analyze.py"
INDEX_JSON="${REPO_ROOT}/skills/INDEX.json"
DEFAULT_DB="data/guardrail_alpha.db"

# Pick the demo DB if it exists; otherwise let analyze.py use its own default.
if [[ -f "${REPO_ROOT}/${DEFAULT_DB}" ]]; then
  DB_ARGS=(--db "${DEFAULT_DB}")
  DB_NOTE="demo event-log database (${DEFAULT_DB})"
else
  DB_ARGS=()
  DB_NOTE="analyze.py default database (no demo db found)"
fi

# --- presentation helpers ---------------------------------------------------

BANNER_NO=0

banner() {
  BANNER_NO=$((BANNER_NO + 1))
  echo
  echo "############################################################"
  printf '#  STEP %d — %s\n' "${BANNER_NO}" "$*"
  echo "############################################################"
}

explain() {
  echo "  » $*"
}

note() {
  echo "  · $*"
}

# Run a command but never let its failure abort the whole walkthrough. Prints a
# clear OK/NON-FATAL line and records whether real output was produced.
STEPS_OK=0
STEPS_SOFT_FAIL=0
guard() {
  local label="$1"
  shift
  echo "  --- running: ${label} ---"
  if "$@"; then
    echo "  --- ${label}: OK ---"
    STEPS_OK=$((STEPS_OK + 1))
  else
    local code=$?
    echo "  --- ${label}: NON-FATAL (exit ${code}) — continuing tour ---" >&2
    STEPS_SOFT_FAIL=$((STEPS_SOFT_FAIL + 1))
  fi
}

# Guard for a python-lab analytics subcommand. Tolerates a missing analyze.py
# or python interpreter without aborting.
analyze() {
  local sub="$1"
  shift
  if [[ ! -f "${ANALYZE_PY}" ]]; then
    note "skipped: python-lab/analyze.py not present in this checkout"
    return 0
  fi
  if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
    note "skipped: ${PYTHON_BIN} not on PATH"
    return 0
  fi
  guard "analyze ${sub}" \
    "${PYTHON_BIN}" "${ANALYZE_PY}" "${sub}" "$@"
}

# ============================================================================
# Intro
# ============================================================================
echo "============================================================"
echo "  GUARDRAIL ALPHA — GUIDED JUDGE WALKTHROUGH"
echo "  offline-safe · no keys · no network · always exits 0"
echo "============================================================"
echo "  Repo: ${REPO_ROOT}"
echo "  Data source for analytics: ${DB_NOTE}"
echo
echo "  This tour runs the REAL tools end to end:"
echo "    1. seed/identify demo data"
echo "    2. analytics: regime · drawdown · montecarlo · ensemble-compare · journal"
echo "    3. build the self-contained HTML report bundle"
echo "    4. lint the strategy skills + print the skill catalog"
echo "    5. independently verify the agent identity / report proof"
echo "    6. print the stress-scenario catalog"

# ============================================================================
# STEP 1 — Demo data
# ============================================================================
banner "Demo data — seed or identify"
explain "Analytics, reports, and proofs all read a local event-log database and"
explain "a run report. If a seeder ships we (re)seed; otherwise we use what's"
explain "already in data/ so the tour stays fully offline."
if [[ -f "${SCRIPT_DIR}/seed_demo_data.sh" ]]; then
  explain "Found scripts/seed_demo_data.sh — seeding deterministic demo data."
  guard "seed_demo_data.sh" bash "${SCRIPT_DIR}/seed_demo_data.sh"
else
  explain "No scripts/seed_demo_data.sh in this checkout — using existing data/."
fi
if [[ -f "${REPO_ROOT}/${DEFAULT_DB}" ]]; then
  note "event-log database present: ${DEFAULT_DB}"
else
  note "no demo database yet — analytics will print clean 'no data' notes (still exit 0)"
fi
if [[ -f "${REPO_ROOT}/data/run_report.json" ]]; then
  note "run report present: data/run_report.json"
fi

# ============================================================================
# STEP 2 — Analytics suite
# ============================================================================
banner "Analytics — regime, drawdown, Monte Carlo, ensemble, journal"
explain "These are the standard-library Python analytics over the run's NAV curve"
explain "and decision events. Each is safe on missing data (prints a note, exits 0)."
echo
explain "2a. Regime analysis — time-in-regime, transition matrix, exposure multipliers."
analyze regime "${DB_ARGS[@]}"
echo
explain "2b. Drawdown — underwater curve, max drawdown, worst episodes."
analyze drawdown "${DB_ARGS[@]}"
echo
explain "2c. Monte Carlo — IID bootstrap VaR/CVaR + P(kill-switch breach)."
analyze montecarlo "${DB_ARGS[@]}"
echo
explain "2d. Ensemble compare (--all) — blended book vs. each single skill."
analyze ensemble-compare --all
echo
explain "2e. Decision journal — human-readable per-cycle reasoning from the log."
analyze journal "${DB_ARGS[@]}"

# ============================================================================
# STEP 3 — HTML report bundle
# ============================================================================
banner "Report bundle — self-contained HTML"
explain "analyze.py bundle composes the dossier, journal, and ensemble views into"
explain "a single browsable folder of inline-CSS HTML (no CDN, no JS deps)."
BUNDLE_INDEX=""
if [[ -f "${ANALYZE_PY}" ]] && command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  # The bundle index path is printed on stdout; capture it while still showing it.
  if BUNDLE_INDEX="$("${PYTHON_BIN}" "${ANALYZE_PY}" bundle "${DB_ARGS[@]}" 2> >(cat >&2))"; then
    STEPS_OK=$((STEPS_OK + 1))
    echo "  --- bundle: OK ---"
    if [[ -n "${BUNDLE_INDEX}" && -f "${REPO_ROOT}/${BUNDLE_INDEX}" ]]; then
      explain "Report bundle landed at: ${REPO_ROOT}/${BUNDLE_INDEX}"
      explain "Open it in a browser to read the full research dossier."
    elif [[ -n "${BUNDLE_INDEX}" ]]; then
      explain "Report bundle index: ${BUNDLE_INDEX}"
    fi
  else
    echo "  --- bundle: NON-FATAL — continuing tour ---" >&2
    STEPS_SOFT_FAIL=$((STEPS_SOFT_FAIL + 1))
  fi
else
  note "skipped: analyze.py / ${PYTHON_BIN} unavailable"
fi

# ============================================================================
# STEP 4 — Skills: lint + catalog
# ============================================================================
banner "Skills — lint examples + print the catalog"
explain "Each strategy skill ships worked examples that are validated by the same"
explain "real loader/validator the agent uses. Then we print the skill catalog."
if [[ -f "${SCRIPT_DIR}/lint_skills.sh" ]]; then
  guard "lint_skills.sh" bash "${SCRIPT_DIR}/lint_skills.sh"
else
  note "skipped: scripts/lint_skills.sh not present in this checkout"
fi
echo
explain "Skill catalog (skills/INDEX.json):"
if [[ -f "${INDEX_JSON}" ]] && command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  guard "skill catalog" "${PYTHON_BIN}" - "${INDEX_JSON}" <<'PYEOF'
import json
import sys

path = sys.argv[1]
try:
    skills = json.load(open(path, encoding="utf-8"))
except Exception as exc:  # noqa: BLE001 - tour must not crash
    print(f"  (could not read skill catalog: {exc})")
    raise SystemExit(0)

if not isinstance(skills, list) or not skills:
    print("  (no skills listed)")
    raise SystemExit(0)

print(f"  {len(skills)} skill(s):")
for i, skill in enumerate(skills, start=1):
    name = skill.get("name") or skill.get("id") or "<unnamed>"
    sid = skill.get("id", "")
    universe = skill.get("eligible_universe_size", "?")
    examples = skill.get("examples_count", "?")
    summary = (skill.get("summary") or "").strip().replace("\n", " ")
    if len(summary) > 140:
        summary = summary[:137] + "..."
    print(f"   [{i}] {name}  (id: {sid})")
    print(f"        universe={universe} tokens · examples={examples}")
    if summary:
        print(f"        {summary}")
PYEOF
else
  note "skipped: skills/INDEX.json or ${PYTHON_BIN} unavailable"
fi

# ============================================================================
# STEP 5 — Identity / proof verification
# ============================================================================
banner "Proof — independently verify the agent identity & report"
explain "A stdlib-only verifier re-derives the policy hash, report hash, and agent"
explain "id from first principles and checks the on-chain contract + explorer URL."
explain "No network: it verifies the live run report if present, else a bundled fixture."
if [[ -f "${SCRIPT_DIR}/verify_proof.sh" ]]; then
  guard "verify_proof.sh" bash "${SCRIPT_DIR}/verify_proof.sh"
else
  note "skipped: scripts/verify_proof.sh not present in this checkout"
fi

# ============================================================================
# STEP 6 — Stress-scenario catalog
# ============================================================================
banner "Scenarios — the stress-test catalog"
explain "Each named scenario stresses one failure mode and declares which guardrail"
explain "protection should fire (throttle / kill-switch / reduce-only / stop-loss)."
if [[ -f "${SCRIPT_DIR}/run_scenarios.sh" ]]; then
  guard "run_scenarios.sh" bash "${SCRIPT_DIR}/run_scenarios.sh"
else
  note "skipped: scripts/run_scenarios.sh not present in this checkout"
fi

# ============================================================================
# What you just saw
# ============================================================================
echo
echo "############################################################"
echo "#  WHAT YOU JUST SAW"
echo "############################################################"
echo "  Steps that produced output: ${STEPS_OK}    soft-skipped/non-fatal: ${STEPS_SOFT_FAIL}"
echo
echo "  • Demo data identified/seeded — fully local, no network."
echo "  • Five analytics over the real run: regime routing, drawdown,"
echo "    Monte-Carlo tail risk, ensemble-vs-single-skill, decision journal."
echo "  • A self-contained HTML report bundle you can open offline."
echo "  • Every strategy skill's examples validated by the real validator,"
echo "    plus the 5-skill catalog."
echo "  • The agent's identity & report proof re-derived and verified offline."
echo "  • The stress-scenario catalog mapping failures -> guardrail responses."
echo
echo "  EXPLORE FURTHER (live, optional):"
if [[ -n "${BUNDLE_INDEX}" ]]; then
  echo "    Report bundle : open ${REPO_ROOT}/${BUNDLE_INDEX} in a browser"
fi
echo "    Web-lite cockpit : bash scripts/serve_cockpit.sh   (static, offline)"
echo "    Full stack       : bash scripts/judge_quickstart.sh (build + API + cockpit)"
echo "    Next.js dashboard: cd dashboard && pnpm install && pnpm dev  -> http://localhost:3000"
echo "    One entry point  : bash scripts/guardrail.sh help"
echo
echo "  Walkthrough complete. (Always exits 0 — every step is guarded.)"
exit 0
