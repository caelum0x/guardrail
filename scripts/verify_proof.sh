#!/usr/bin/env bash
#
# verify_proof.sh — independent, offline verification of the Guardrail agent's
# BNB identity / report proof.
#
# Runs the stdlib-only Python verifier (clients/proof-verifier/verify.py) which
# re-derives the agent's policy_hash, report_hash, and agent_id from first
# principles and checks the competition contract + explorer URL formats.
#
# Target selection:
#   1. an explicit proof path passed as $1, otherwise
#   2. data/run_report.json if it exists, otherwise
#   3. the bundled offline fixture clients/proof-verifier/sample_proof.json
#
# Exits 0 only when every applicable check passes. No network or keys required.

set -euo pipefail

# Resolve the repository root from this script's location so the tool works
# regardless of the caller's current directory.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

VERIFIER="${REPO_ROOT}/clients/proof-verifier/verify.py"
RUN_REPORT="${REPO_ROOT}/data/run_report.json"
SAMPLE_FIXTURE="${REPO_ROOT}/clients/proof-verifier/sample_proof.json"

if [[ ! -f "${VERIFIER}" ]]; then
  echo "error: verifier not found at ${VERIFIER}" >&2
  exit 2
fi

PYTHON_BIN="${PYTHON_BIN:-python3}"
if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
  echo "error: ${PYTHON_BIN} is required but not on PATH" >&2
  exit 2
fi

# Decide which proof document to verify.
if [[ $# -ge 1 && -n "${1:-}" ]]; then
  PROOF_PATH="$1"
  echo "Verifying explicit proof: ${PROOF_PATH}"
elif [[ -f "${RUN_REPORT}" ]]; then
  PROOF_PATH="${RUN_REPORT}"
  echo "Verifying live run report: ${PROOF_PATH}"
else
  PROOF_PATH="${SAMPLE_FIXTURE}"
  echo "No run report found; verifying bundled offline fixture: ${PROOF_PATH}"
fi

echo
exec "${PYTHON_BIN}" "${VERIFIER}" "${PROOF_PATH}"
