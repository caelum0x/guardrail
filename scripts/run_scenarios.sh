#!/usr/bin/env bash
# ============================================================================
# Guardrail Alpha — STRESS SCENARIO RUNNER
# ============================================================================
# Lists the named stress-scenario library (configs/scenarios/*.json) and, for
# each scenario, prints what it stresses and which guardrail protection is
# expected to fire (throttle / kill switch / reduce-only / stop-loss).
#
# This is a judge-facing, OFFLINE-SAFE walkthrough: it reads only local JSON,
# requires no network, keys, or running services, and exits 0 on success.
#
# If the guardrail-sim binary is present it ALSO prints the exact command to
# replay the scenario through the real backtest + risk + portfolio path, but it
# never builds or runs anything network-dependent.
# ============================================================================
set -euo pipefail

# Resolve repo root from this script's location (works from any cwd).
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCENARIO_DIR="${ROOT}/configs/scenarios"
INDEX="${SCENARIO_DIR}/index.json"

# --- helpers ---------------------------------------------------------------

# Read a top-level string field from a scenario JSON file via python3 (always
# available here) with a jq fallback. Offline, no third-party deps.
json_field() {
  local file="$1" field="$2"
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "import json,sys; print(json.load(open(sys.argv[1])).get(sys.argv[2],''))" "$file" "$field"
  elif command -v jq >/dev/null 2>&1; then
    jq -r --arg f "$field" '.[$f] // ""' "$file"
  else
    echo ""
  fi
}

# Iterate the index entries as "id|label|file|expected_response" lines.
index_rows() {
  if command -v python3 >/dev/null 2>&1; then
    python3 -c "
import json,sys
idx=json.load(open(sys.argv[1]))
for e in idx:
    print('|'.join([str(e.get('id','')), str(e.get('label','')), str(e.get('file','')), str(e.get('expected_response',''))]))
" "$INDEX"
  elif command -v jq >/dev/null 2>&1; then
    jq -r '.[] | [.id, .label, .file, .expected_response] | join("|")' "$INDEX"
  else
    echo "ERROR: need python3 or jq to read ${INDEX}" >&2
    exit 1
  fi
}

# Map an expected_response token to a human-readable protection description.
response_desc() {
  case "$1" in
    kill_switch) echo "KILL SWITCH — halt all risk, go reduce-only/flat (drawdown >= kill_switch_drawdown_pct)" ;;
    throttle)    echo "THROTTLE — soft/hard exposure throttle, block new entries (drawdown >= throttle/max_*_drawdown_pct)" ;;
    reduce_only) echo "REDUCE-ONLY — cap or reject new risk, hold/trim existing (slippage/liquidity/funding/position limits)" ;;
    stop_loss)   echo "STOP-LOSS — exit the offending position at the policy stop" ;;
    *)           echo "UNKNOWN expected_response: $1" ;;
  esac
}

# --- preflight -------------------------------------------------------------

if [[ ! -f "${INDEX}" ]]; then
  echo "ERROR: scenario index not found: ${INDEX}" >&2
  exit 1
fi

echo "============================================================"
echo "  Guardrail Alpha — Stress Scenario Library"
echo "  index: ${INDEX}"
echo "============================================================"
echo "  Risk policy under test (configs/risk_policy.production.json):"
echo "    max_total_drawdown_pct   = 22"
echo "    max_daily_drawdown_pct   = 7"
echo "    max_new_position_pct     = 12"
echo "    max_slippage_pct         = 0.8"
echo "    kill_switch_drawdown_pct = 24"
echo "============================================================"

# Detect whether a guardrail-sim binary exists (compiled or via cargo).
SIM_HINT=""
if [[ -x "${ROOT}/target/release/guardrail-sim" ]]; then
  SIM_HINT="${ROOT}/target/release/guardrail-sim --policy configs/risk_policy.production.json"
elif [[ -x "${ROOT}/target/debug/guardrail-sim" ]]; then
  SIM_HINT="${ROOT}/target/debug/guardrail-sim --policy configs/risk_policy.production.json"
elif command -v cargo >/dev/null 2>&1 && [[ -d "${ROOT}/apps/guardrail-sim" ]]; then
  SIM_HINT="cargo run -p guardrail-sim -- --policy configs/risk_policy.production.json"
fi

# --- walk the library ------------------------------------------------------

count=0
missing=0
while IFS='|' read -r id label file expected; do
  [[ -z "${id}" ]] && continue
  count=$((count + 1))
  path="${SCENARIO_DIR}/${file}"

  echo
  echo "------------------------------------------------------------"
  printf '  [%d] %s  (id: %s)\n' "${count}" "${label}" "${id}"
  echo "------------------------------------------------------------"

  if [[ ! -f "${path}" ]]; then
    echo "  MISSING scenario file: ${path}" >&2
    missing=$((missing + 1))
    continue
  fi

  desc="$(json_field "${path}" description)"
  detail="$(json_field "${path}" expected_detail)"
  echo "  file:        configs/scenarios/${file}"
  echo "  tests:       ${desc}"
  echo "  EXPECTED:    $(response_desc "${expected}")"
  if [[ -n "${detail}" ]]; then
    echo "  why:         ${detail}"
  fi

  if [[ -n "${SIM_HINT}" ]]; then
    echo "  replay (sentiment sweep through real risk path):"
    echo "      ${SIM_HINT}"
  else
    echo "  replay:      guardrail-sim not built; build with 'cargo build -p guardrail-sim'"
  fi
done < <(index_rows)

echo
echo "============================================================"
echo "  Scenarios listed: ${count}   missing files: ${missing}"
echo "============================================================"

if [[ "${missing}" -ne 0 ]]; then
  echo "ERROR: one or more scenario files referenced by the index are missing." >&2
  exit 1
fi

exit 0
