#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:8080}"

curl -fsS "$API_URL/health"
echo
curl -fsS "$API_URL/cockpit" >/dev/null
curl -fsS "$API_URL/alerts" >/dev/null
curl -fsS "$API_URL/readiness" >/dev/null
curl -fsS "$API_URL/policy" >/dev/null
curl -fsS "$API_URL/universe" >/dev/null
curl -fsS "$API_URL/ops" >/dev/null

echo "guardrail api healthy: $API_URL"
