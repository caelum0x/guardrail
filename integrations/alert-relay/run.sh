#!/usr/bin/env bash
#
# Offline-safe wrapper around the Guardrail alert relay.
#
# By DEFAULT this runs a single dry-run poll (--once --dry-run): it makes no
# network calls to any sink and needs no secrets, so it is safe to run anywhere
# (CI, a fresh checkout, a cron smoke probe). Pass extra flags to override.
#
# Examples:
#   ./run.sh                          # single offline dry-run poll (default)
#   ./run.sh --live                   # single live poll (real delivery)
#   ./run.sh --live --config x.json   # live, custom config
#   RELAY_ARGS="--live" ./run.sh      # same, via env (useful in containers)
#
# Any arguments passed on the command line REPLACE the defaults entirely, so
# you stay in full control of dry-run vs live.
set -euo pipefail

# Resolve this script's directory so the relay is found regardless of cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELAY="${SCRIPT_DIR}/relay.py"

# Allow choosing the interpreter (defaults to python3).
PYTHON_BIN="${PYTHON_BIN:-python3}"

# Default to the offline-safe mode. Callers can override by passing their own
# arguments, or by setting RELAY_ARGS in the environment.
DEFAULT_ARGS=(--once --dry-run)

if [ "$#" -gt 0 ]; then
  # Explicit CLI args take precedence over everything.
  ARGS=("$@")
elif [ -n "${RELAY_ARGS:-}" ]; then
  # Word-split RELAY_ARGS intentionally so "--once --dry-run" becomes two args.
  # shellcheck disable=SC2206
  ARGS=(${RELAY_ARGS})
else
  ARGS=("${DEFAULT_ARGS[@]}")
fi

exec "${PYTHON_BIN}" "${RELAY}" "${ARGS[@]}"
