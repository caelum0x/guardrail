#!/usr/bin/env bash
#
# seed_demo_data.sh — generate a rich, deterministic demo dataset for the
# Guardrail Alpha analytics, then print the commands to explore it.
#
# This writes ONLY to a separate demo location
# (data/demo_guardrail_alpha.db + data/demo_run_report.json) and never touches
# the real data/guardrail_alpha.db or data/run_report.json.
#
# Usage:
#   scripts/seed_demo_data.sh [CYCLES] [SEED]
#
# Examples:
#   scripts/seed_demo_data.sh           # defaults (40 cycles, default seed)
#   scripts/seed_demo_data.sh 60        # 60 cycles
#   scripts/seed_demo_data.sh 50 1234   # 50 cycles, seed 1234

set -euo pipefail

# Resolve the repository root from this script's location so the command works
# regardless of the caller's working directory.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

PYTHON_BIN="${PYTHON_BIN:-python3}"
ANALYZE="python-lab/analyze.py"

DEMO_DB="data/demo_guardrail_alpha.db"
DEMO_REPORT="data/demo_run_report.json"

CYCLES="${1:-40}"
SEED="${2:-20260614}"

echo "==> Seeding demo dataset (cycles=${CYCLES}, seed=${SEED})"
echo "    Demo DB:     ${DEMO_DB}"
echo "    Demo report: ${DEMO_REPORT}"
echo "    (real data/guardrail_alpha.db and data/run_report.json are untouched)"
echo

"${PYTHON_BIN}" "${ANALYZE}" seed \
  --db "${DEMO_DB}" \
  --report "${DEMO_REPORT}" \
  --cycles "${CYCLES}" \
  --seed "${SEED}"

echo
echo "==> Demo dataset ready. Try the analytics against it:"
echo
echo "  ${PYTHON_BIN} ${ANALYZE} regime           --db ${DEMO_DB}"
echo "  ${PYTHON_BIN} ${ANALYZE} drawdown         --db ${DEMO_DB}"
echo "  ${PYTHON_BIN} ${ANALYZE} montecarlo --paths 200 --db ${DEMO_DB}"
echo "  ${PYTHON_BIN} ${ANALYZE} journal          --db ${DEMO_DB}"
echo "  ${PYTHON_BIN} ${ANALYZE} ensemble-compare --all --db ${DEMO_DB}"
echo "  ${PYTHON_BIN} ${ANALYZE} dossier          --db ${DEMO_DB} --report ${DEMO_REPORT}"
echo "  ${PYTHON_BIN} ${ANALYZE} bundle --out /tmp/demo_reports --db ${DEMO_DB} --report ${DEMO_REPORT}"
echo
