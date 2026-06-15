#!/usr/bin/env bash
#
# self_custody_demo.sh — narrated, OFFLINE walkthrough of Guardrail's TWAK
# self-custody flow for the TWAK prize.
#
#   agent proposes  ->  risk engine gates  ->  TWAK signs with user-held keys
#                    ->  execution + reconcile
#
# This script never touches the network and never loads or requires any key
# material. It prints each step of the flow and points at the REAL files and
# HTTP routes in this repo that enforce the property, so a reviewer can read the
# source of truth directly.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." >/dev/null 2>&1 && pwd)"

# Colors only when stdout is a TTY; degrade gracefully in pipes/CI.
if [[ -t 1 ]]; then
  BOLD="$(printf '\033[1m')"; DIM="$(printf '\033[2m')"; RESET="$(printf '\033[0m')"
  CYAN="$(printf '\033[36m')"; GREEN="$(printf '\033[32m')"; YELLOW="$(printf '\033[33m')"
else
  BOLD=""; DIM=""; RESET=""; CYAN=""; GREEN=""; YELLOW=""
fi

hr() { printf '%s\n' "------------------------------------------------------------"; }

section() {
  printf '\n%s%s%s\n' "${BOLD}${CYAN}" "$1" "${RESET}"
  hr
}

step() {
  printf '%s>>%s %s\n' "${GREEN}${BOLD}" "${RESET}" "$1"
}

note() {
  printf '   %s%s%s\n' "${DIM}" "$1" "${RESET}"
}

# Point at a repo file and show it exists (read-only; no content is required).
ref() {
  local path="$1" desc="$2"
  if [[ -e "${REPO_ROOT}/${path}" ]]; then
    printf '   %ssource:%s %s  %s(%s)%s\n' "${YELLOW}" "${RESET}" "${path}" "${DIM}" "${desc}" "${RESET}"
  else
    printf '   %ssource:%s %s  %s(%s — not present in this checkout)%s\n' \
      "${YELLOW}" "${RESET}" "${path}" "${DIM}" "${desc}" "${RESET}"
  fi
}

route() {
  printf '   %sroute:%s  %-22s %s%s%s\n' "${YELLOW}" "${RESET}" "$1" "${DIM}" "$2" "${RESET}"
}

# ---------------------------------------------------------------------------

printf '%s\n' "============================================================"
printf '%s%s%s\n' "${BOLD}" " Guardrail x TWAK — Self-Custody Demo (offline)" "${RESET}"
printf '%s\n' "============================================================"
note "No network. No keys. Keys + signing stay with the user's wallet (TWAK)."
note "repo root: ${REPO_ROOT}"

# ---------------------------------------------------------------------------
section "0. The invariant"
cat <<'TXT'
   Nothing in the workspace except the TWAK client can produce a signature or
   an on-chain trade — and even the TWAK client only executes an order the risk
   engine has already approved. The agent PROPOSES; it can never sign or move
   funds. This is enforced by the type system and the dependency graph, not by
   convention.
TXT
ref "docs/SELF_CUSTODY.md" "the self-custody design, end to end"
ref "crates/twak-client/src/lib.rs" "TwakExecutor trait — the only execution boundary"

# ---------------------------------------------------------------------------
section "1. Agent PROPOSES an intent (no keys, no signing)"
step "Strategy builds an OrderIntent — plain data describing WHAT should happen."
note "An OrderIntent carries no signing authority. Quoting is authority-free."
ref "crates/agent-runtime/src/runtime.rs" "strategy loop builds OrderIntents"
ref "crates/twak-client/src/quote.rs" "quote_swap(intent) — read-only, no keys"
route "/signals" "latest regime + portfolio target the proposal is based on"
route "/quotes"  "quote evidence captured before any swap"

# ---------------------------------------------------------------------------
section "2. Risk engine GATES the proposal (the only door to TWAK)"
step "RiskEngine pre-trade + approve(quote) either mints an ApprovedOrder or rejects/clips."
note "ApprovedOrder has NO public constructor: 'execute without approval' is a"
note "compile error, not a runtime check. require_quote_before_swap = true."
ref "crates/twak-client/src/risk.rs"      "TOKEN_RISK_CHECK_REQUIRED = true"
ref "configs/risk_policy.production.json"  "caps, slippage bound, forbidden_actions"
ref "configs/signing_policy.example.json"  "example TWAK authorization envelope"
route "/risk"     "RiskApproved / RiskRejected / RiskClipped decisions"
route "/policy"   "live policy + enforcement (execution_layer=twak_only)"

printf '\n'
step "Authorization envelope the user's wallet enforces (from the example policy):"
SIGNING_POLICY="${REPO_ROOT}/configs/signing_policy.example.json"
if command -v python3 >/dev/null 2>&1 && [[ -f "${SIGNING_POLICY}" ]]; then
  python3 - "${SIGNING_POLICY}" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    p = json.load(fh)
caps = p.get("caps", {})
per_tx = caps.get("per_tx", {})
daily = caps.get("daily", {})
slip = p.get("slippage", {})
custody = p.get("custody", {})
print(f"     custody model      : {custody.get('model')} (signer={custody.get('signer')}, agent_can_sign={custody.get('agent_can_sign')})")
print(f"     per-tx cap         : {per_tx.get('max_notional_usd')} USD, max_position_pct={per_tx.get('max_position_pct')}")
print(f"     daily cap          : {daily.get('max_total_notional_usd')} USD over <= {daily.get('max_tx_count')} txs")
print(f"     slippage bound     : <= {slip.get('max_slippage_pct')}%  (quote_before_swap={slip.get('require_quote_before_swap')})")
print(f"     allowed actions    : {', '.join(p.get('allowed_actions', []))}")
print(f"     forbidden actions  : {', '.join(p.get('forbidden_actions', []))}")
PY
else
  note "(python3 not available — open configs/signing_policy.example.json to read the caps)"
fi

# ---------------------------------------------------------------------------
section "3. TWAK SIGNS with the user-held keys"
step "Only TWAK signs and broadcasts; it admits an ApprovedOrder and nothing else."
note "execute_swap(approved: &ApprovedOrder) is the sole funds-moving method."
note "For x402-gated data, the agent builds the payload but TWAK signs it."
ref "crates/twak-client/src/x402.rs"     "sign_authorization — signing lives in TWAK"
ref "crates/twak-client/src/swap.rs"     "execute_swap -> TxReceipt"
ref "crates/twak-client/src/mock.rs"     "offline MockTwakClient (default: no keys/network)"
route "/signing-policy" "x402 authorization is signed by TWAK, not the agent"

printf '\n'
step "Offline signer demonstration (deterministic mock — NOT real key material):"
if command -v python3 >/dev/null 2>&1; then
  python3 - <<'PY'
import hashlib
# Mirrors crates/twak-client/src/x402.rs::sign_authorization:
#   sha256(signer || 0x00 || authorization), 0x-prefixed hex.
signer = "0xA9e5C0FfEe0000000000000000000000000A1b2C3"  # user wallet (held by TWAK)
authorization = '{"action":"execute_swap","asset":"ETH","max_slippage_pct":"0.8"}'
digest = hashlib.sha256(signer.encode() + b"\x00" + authorization.encode()).hexdigest()
print(f"     signer (user wallet): {signer}")
print(f"     authorization       : {authorization}")
print(f"     signature           : 0x{digest}")
print("     NOTE: the agent process never sees a private key; TWAK holds it.")
PY
else
  note "(python3 not available — see crates/twak-client/src/x402.rs for the signer)"
fi

# ---------------------------------------------------------------------------
section "4. EXECUTION + reconcile (auditable, read-only surfaces)"
step "TWAK returns a TxReceipt; the runtime reconciles the portfolio and logs events."
note "The API is read-only and the dashboard cannot call TWAK — least privilege"
note "is enforced by the dependency graph."
ref "data/run_report.json"  "the run report committed to via policy_hash/report_hash"
route "/proof"      "judge-facing proof (agent_id, hashes, BscScan URLs)"
route "/readiness"  "transaction proof + daily-trade checks"
route "/compete"    "competition contract registration status"

printf '\n'
step "Verify the resulting proof independently (offline, stdlib-only):"
note "./scripts/verify_proof.sh   # re-derives policy_hash/report_hash, checks URLs"

# ---------------------------------------------------------------------------
section "Self-custody summary (TWAK prize criteria)"
cat <<'TXT'
   [x] Keys remain with the user's wallet; the agent holds none.
   [x] Signing authority remains with TWAK; the agent only proposes.
   [x] Risk engine is the only gate; ApprovedOrder is unforgeable (type-enforced).
   [x] Per-tx + daily caps, slippage bound, allow/forbid lists are policy-declared.
   [x] custodial_signing / bypass_twak / key_export are forbidden actions.
   [x] Same shape offline (MockTwakClient) and live (REST/MCP/CLI transports).
TXT

printf '\n%s%s%s\n' "${GREEN}${BOLD}" "Self-custody demo complete — no keys used, no network touched." "${RESET}"
exit 0
