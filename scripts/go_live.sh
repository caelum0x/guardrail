#!/usr/bin/env bash
#
# go_live.sh — the single, turn-key live go-live command.
#
# THIS SPENDS REAL MONEY. It registers the agent on-chain and trades live on BSC
# through TWAK. It is the one human-in-the-loop step the engineering cannot do
# for you: it requires YOUR keys and YOUR funded wallet, supplied via the
# environment (never committed).
#
# Required environment (see .env.example):
#   CMC_API_KEY        real CoinMarketCap API key
#   BSC_RPC_URL        a BSC mainnet JSON-RPC endpoint
#   TWAK_REST_URL or TWAK_MCP_URL   live TWAK transport (funded, self-custody wallet)
#
# Steps:
#   1. Preflight: guardrail-doctor --live (credentials, live config, safe caps).
#   2. Chain reachability: independent on-chain verifier against BSC_RPC_URL.
#   3. Run the agent live (bounded cycles) -> on-chain registration + real swaps.
#   4. Capture proof: export submission.md, verify on-chain, tick the checklist.
#   5. Print BscScan links for the registration tx and wallet.
#
# Usage:
#   scripts/go_live.sh            # interactive confirmation
#   scripts/go_live.sh --yes      # skip the confirmation prompt (CI/automation)
#   CYCLES=10 scripts/go_live.sh   # number of live cycles (default 6)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

PY="${PYTHON_BIN:-python3}"
CYCLES="${CYCLES:-6}"
ASSUME_YES=0
[[ "${1:-}" == "--yes" || "${GUARDRAIL_GO_LIVE_YES:-}" == "1" ]] && ASSUME_YES=1

echo "============================================================"
echo " Guardrail Alpha — LIVE go-live"
echo " THIS SPENDS REAL MONEY and trades live on BSC."
echo "============================================================"

# --- 1. Preflight ------------------------------------------------------------
echo "==> 1/5  Live preflight (guardrail-doctor --live)"
if ! cargo run -q -p guardrail-doctor -- --live; then
  echo "Preflight failed — fix the FAILs above before going live." >&2
  exit 1
fi

# --- 2. Chain reachability ---------------------------------------------------
echo "==> 2/5  On-chain reachability (verifier against BSC_RPC_URL)"
"${PY}" clients/proof-verifier/verify.py --rpc "${BSC_RPC_URL}" || {
  echo "On-chain verifier reported issues — inspect above before continuing." >&2
  exit 1
}

# --- Confirmation ------------------------------------------------------------
if [[ "${ASSUME_YES}" -ne 1 ]]; then
  echo
  read -r -p "Proceed with LIVE trading for ${CYCLES} cycles? Type 'GO LIVE' to confirm: " reply
  if [[ "${reply}" != "GO LIVE" ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# --- 3. Live run -------------------------------------------------------------
echo "==> 3/5  Running agent LIVE (${CYCLES} cycles)"
APP_ENV=live GUARDRAIL_CYCLES="${CYCLES}" \
  cargo run -q -p guardrail-agent -- --config configs/production.toml

# --- 4. Capture proof --------------------------------------------------------
echo "==> 4/5  Capturing proof"
OUT_DIR="data/exports" "${SCRIPT_DIR}/export_report.sh" || true
"${PY}" clients/proof-verifier/verify.py --rpc "${BSC_RPC_URL}" || true
"${PY}" python-lab/scripts/tick_checklist.py

# --- 5. Proof links ----------------------------------------------------------
echo "==> 5/5  Proof"
REPORT="data/run_report.json"
if [[ -f "${REPORT}" ]]; then
  "${PY}" - "$REPORT" <<'PYEOF'
import json, sys
d = json.load(open(sys.argv[1]))
tx = d.get("registration_tx")
wallet = d.get("wallet_address", "")
if tx:
    print(f"  registration tx : https://bscscan.com/tx/{tx}")
else:
    print("  registration tx : (none recorded — check the run log)")
if wallet:
    print(f"  wallet          : https://bscscan.com/address/{wallet}")
PYEOF
fi

echo
echo "==> Live go-live complete. Review docs/SUBMISSION_CHECKLIST.md (should now"
echo "    tick Competition registration with a real on-chain tx)."
