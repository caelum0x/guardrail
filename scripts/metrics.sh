#!/usr/bin/env bash
# Scrape Guardrail Alpha metrics in Prometheus text format.
set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:8080}"
curl -fsS "${API_URL}/metrics"

if [[ -n "${EXPORTER_URL:-}" ]]; then
  printf "\n# --- exporter: %s ---\n" "${EXPORTER_URL}"
  curl -fsS "${EXPORTER_URL}/metrics"
fi
