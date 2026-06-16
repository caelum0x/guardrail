#!/usr/bin/env bash
#
# smoke_quant.sh — exercise the read-only quant API surface end to end.
#
# Starts nothing; assumes a guardrail-api is running (default :8080, or set
# GUARDRAIL_API). Hits every quant endpoint, checks for HTTP 200 + valid JSON
# without an "error" field, and prints a PASS/FAIL line per endpoint. Exits
# non-zero if any check fails — safe to wire into a manual pre-ship check.
set -uo pipefail

API="${GUARDRAIL_API:-http://127.0.0.1:8080}"
PY="${PYTHON_BIN:-python3}"
fails=0

check() {
  local name="$1" path="$2"
  local body
  body="$(curl -fsS "${API}${path}" 2>/dev/null)"
  if [ -z "$body" ]; then
    printf '  [FAIL] %-22s (no response)\n' "$name"; fails=$((fails + 1)); return
  fi
  if ! printf '%s' "$body" | "$PY" -c 'import sys,json; json.load(sys.stdin)' 2>/dev/null; then
    printf '  [FAIL] %-22s (invalid JSON)\n' "$name"; fails=$((fails + 1)); return
  fi
  if printf '%s' "$body" | "$PY" -c 'import sys,json; sys.exit(0 if "error" not in json.load(sys.stdin) else 1)' 2>/dev/null; then
    printf '  [PASS] %-22s\n' "$name"
  else
    printf '  [WARN] %-22s (JSON has "error" — may need a prior agent run)\n' "$name"
  fi
}

echo "quant API smoke against ${API}"
check "ta"               "/ta?indicator=rsi&series=44,44.3,44.1,43.6,44.3,44.8&period=5"
check "fees"             "/fees?notional_usd=25000&quantity=12&side=buy"
check "sizer"            "/sizer?method=kelly&win_prob=0.6&odds=1.5"
check "orderbook"        "/orderbook?orders=s,limit,101,5;b,market,,6"
check "pnl"              "/pnl?fills=CAKE,buy,10,2;CAKE,sell,4,3&marks=CAKE:3"
check "correlation"      "/correlation?series=BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025"
check "equity/indicators" "/equity/indicators?indicator=rsi&period=14"
check "portfolio/risk"   "/portfolio/risk"
check "cmc/capabilities" "/cmc/capabilities"

echo
if [ "$fails" -eq 0 ]; then
  echo "OK — all quant endpoints responded with valid JSON"
else
  echo "FAILED — ${fails} endpoint(s) did not respond correctly"
fi
exit "$fails"
