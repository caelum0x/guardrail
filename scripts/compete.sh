#!/usr/bin/env bash
# ============================================================================
# Guardrail Alpha — LIVE COMPETITION LAUNCHER
# ============================================================================
# One command to take the agent live for the competition:
#
#   1. Preflight readiness (guardrail-doctor)
#   2. Register the agent for the competition via TWAK (REST, autonomous)
#   3. Print the competition contract
#   4. Start the live trading agent against configs/production.toml
#
# The runtime falls back to deterministic mocks when keys/URLs are absent, so
# this script is safe to dry-run, but a real live run REQUIRES the env below.
# See docs/LIVE_RUNBOOK.md for the full operator runbook.
# ============================================================================
set -euo pipefail

cd "$(dirname "$0")/.."

CONFIG="${GUARDRAIL_CONFIG:-configs/production.toml}"
COMPETITION_CONTRACT="0x212c61b9b72c95d95bf29cf032f5e5635629aed5"

section() {
  echo
  echo "============================================================"
  echo "» $*"
  echo "============================================================"
}

# Print whether a required/optional env var is set, without leaking the value.
checkenv() {
  local name="$1" kind="$2"
  if [ -n "${!name:-}" ]; then
    echo "  [ok]      $name is set"
  elif [ "$kind" = "required" ]; then
    echo "  [MISSING] $name is NOT set ($kind) — agent will fall back to mock"
  else
    echo "  [skip]    $name is not set ($kind)"
  fi
}

section "0. Environment checklist"
echo "Config: $CONFIG"
echo
echo "Required for a real live run:"
checkenv CMC_API_KEY required
checkenv TWAK_BASE_URL required
checkenv BSC_RPC_URL required
echo
echo "Optional (x402-settled CMC requests):"
checkenv CMC_X402_FROM optional
checkenv CMC_X402_SIGNATURE optional
echo
echo "If any required var is missing the agent stays on deterministic mocks"
echo "(safe, offline). Export them to go fully live."

section "1. Preflight readiness (guardrail-doctor)"
cargo run -q -p guardrail-doctor

section "2. Register the agent for the competition (TWAK REST, autonomous)"
echo "Registering via REST transport. If TWAK_BASE_URL is unset the CLI falls"
echo "back to the offline mock and prints the manual 'twak compete register'"
echo "self-custody fallback you can run by hand."
cargo run -q -p guardrail-cli -- register --transport rest --autonomous true

section "3. Competition contract"
echo "competition_contract: $COMPETITION_CONTRACT"

section "4. Start the live trading agent"
echo "Launching guardrail-agent against $CONFIG."
echo "Stop with Ctrl-C; trigger the kill switch with:"
echo "  cargo run -q -p guardrail-cli -- kill-switch --reason \"operator stop\""
echo
exec cargo run -p guardrail-agent -- --config "$CONFIG"
