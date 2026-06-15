#!/usr/bin/env bash
# ============================================================================
# guardrail.sh — single entry-point dispatcher for Guardrail Alpha
# ============================================================================
# One command surface that delegates to the existing offline-safe tools in
# scripts/ and python-lab/. Every subcommand is a thin, guarded wrapper: if the
# underlying tool is missing, it explains what's wrong and exits non-zero rather
# than failing obscurely.
#
#   scripts/guardrail.sh <command> [args...]
#
# Commands:
#   up                          Build + run + serve the judge quickstart stack.
#   cockpit [port]              Serve the web-lite cockpit (static, offline).
#   analyze <sub> [args...]     Run python-lab analytics (regime|drawdown|
#                               montecarlo|dossier|ensemble|ensemble-compare|
#                               journal).
#   scenarios                   Walk the stress-scenario library.
#   verify [proof.json]         Independently verify the agent's report proof.
#   alerts                      Single dry-run alert relay poll (no network).
#   skills                      Lint every skill's examples with the real
#                               validator.
#   new-skill <name>            Scaffold a new Track-2 strategy skill.
#   help | -h | --help          Show this panel.
#
# Everything runs in paper / offline mode: no API keys, no network required.
# ============================================================================
set -euo pipefail

# Resolve repo root from this script's location so it works from any cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

PYTHON_BIN="${PYTHON_BIN:-python3}"

ANALYZE_PY="${REPO_ROOT}/python-lab/analyze.py"
RELAY_PY="${REPO_ROOT}/integrations/alert-relay/relay.py"

# --- helpers ---------------------------------------------------------------

# Print usage panel (extracted from the header comment so it stays in sync).
usage() {
  sed -n '4,27p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

err() {
  echo "error: $*" >&2
}

# Require an executable script to exist; explain + exit if not.
require_script() {
  local path="$1" label="$2"
  if [[ ! -f "${path}" ]]; then
    err "${label} not found at ${path}"
    err "this command is unavailable in this checkout."
    exit 1
  fi
}

# Require a Python file to exist.
require_pyfile() {
  local path="$1" label="$2"
  if [[ ! -f "${path}" ]]; then
    err "${label} not found at ${path}"
    err "this command is unavailable in this checkout."
    exit 1
  fi
  if ! command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
    err "${PYTHON_BIN} is required but not on PATH"
    exit 1
  fi
}

# --- subcommand implementations --------------------------------------------

cmd_up() {
  require_script "${SCRIPT_DIR}/judge_quickstart.sh" "judge_quickstart.sh"
  exec bash "${SCRIPT_DIR}/judge_quickstart.sh" "$@"
}

cmd_cockpit() {
  require_script "${SCRIPT_DIR}/serve_cockpit.sh" "serve_cockpit.sh"
  exec bash "${SCRIPT_DIR}/serve_cockpit.sh" "$@"
}

cmd_analyze() {
  require_pyfile "${ANALYZE_PY}" "python-lab/analyze.py"
  if [[ $# -lt 1 ]]; then
    err "analyze needs a subcommand: regime|drawdown|montecarlo|dossier|ensemble|ensemble-compare|journal"
    exit 2
  fi
  exec "${PYTHON_BIN}" "${ANALYZE_PY}" "$@"
}

cmd_scenarios() {
  require_script "${SCRIPT_DIR}/run_scenarios.sh" "run_scenarios.sh"
  exec bash "${SCRIPT_DIR}/run_scenarios.sh" "$@"
}

cmd_verify() {
  require_script "${SCRIPT_DIR}/verify_proof.sh" "verify_proof.sh"
  exec bash "${SCRIPT_DIR}/verify_proof.sh" "$@"
}

cmd_alerts() {
  require_pyfile "${RELAY_PY}" "integrations/alert-relay/relay.py"
  # Always a single, dry-run poll: offline-safe, no sink network calls.
  exec "${PYTHON_BIN}" "${RELAY_PY}" --once --dry-run "$@"
}

cmd_skills() {
  require_script "${SCRIPT_DIR}/lint_skills.sh" "lint_skills.sh"
  exec bash "${SCRIPT_DIR}/lint_skills.sh" "$@"
}

cmd_new_skill() {
  require_script "${SCRIPT_DIR}/new_skill.sh" "new_skill.sh"
  if [[ $# -lt 1 ]]; then
    err "new-skill needs a name, e.g. scripts/guardrail.sh new-skill funding-skew-bsc"
    exit 2
  fi
  exec bash "${SCRIPT_DIR}/new_skill.sh" "$@"
}

# --- dispatch --------------------------------------------------------------

main() {
  if [[ $# -lt 1 ]]; then
    usage
    exit 0
  fi

  local cmd="$1"
  shift

  case "${cmd}" in
    up)            cmd_up "$@" ;;
    cockpit)       cmd_cockpit "$@" ;;
    analyze)       cmd_analyze "$@" ;;
    scenarios)     cmd_scenarios "$@" ;;
    verify)        cmd_verify "$@" ;;
    alerts)        cmd_alerts "$@" ;;
    skills)        cmd_skills "$@" ;;
    new-skill)     cmd_new_skill "$@" ;;
    help|-h|--help) usage ;;
    *)
      err "unknown command: ${cmd}"
      echo >&2
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
