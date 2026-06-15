#!/usr/bin/env bash
#
# capture_submission.sh — run the agent and capture all Track-1 submission proof
# in one command, then auto-tick docs/SUBMISSION_CHECKLIST.md from real evidence.
#
# Offline-safe: defaults to paper/mock (no keys, no network). When the live
# environment is configured (APP_ENV=live + real keys), point CONFIG at the
# production config to capture against real services instead — see
# scripts/go_live.sh.
#
# Steps:
#   1. Run the agent (paper) -> rich event log + run_report.json.
#   2. Run a low-threshold kill-switch demo -> a real KillSwitchTriggered event
#      (shared DB, throwaway report so the primary report is preserved).
#   3. Export submission markdown (data/exports/submission.md).
#   4. Independently verify the proof (clients/proof-verifier).
#   5. Auto-tick the submission checklist from the captured evidence.
#
# Usage: scripts/capture_submission.sh [CONFIG]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

CONFIG="${1:-configs/paper.toml}"
PY="${PYTHON_BIN:-python3}"

echo "==> 1/5  Running agent: ${CONFIG}"
cargo run -q -p guardrail-agent -- --config "${CONFIG}"

echo "==> 2/5  Kill-switch demonstration (low-threshold, shared DB)"
GUARDRAIL_REPORT="data/killswitch_demo_report.json" \
  cargo run -q -p guardrail-agent -- --config configs/killswitch_demo.toml \
  || echo "    (kill-switch demo run returned non-zero; continuing)"

echo "==> 3/5  Exporting submission markdown"
OUT_DIR="data/exports" "${SCRIPT_DIR}/export_report.sh" || \
  echo "    (export_report.sh fell back / API not running; run report still on disk)"

echo "==> 4/5  Independent proof verification"
"${PY}" clients/proof-verifier/verify.py || \
  echo "    (verifier reported issues — inspect above)"

echo "==> 5/5  Ticking submission checklist from evidence"
"${PY}" python-lab/scripts/tick_checklist.py

echo
echo "==> Done. Review docs/SUBMISSION_CHECKLIST.md and data/exports/submission.md"
