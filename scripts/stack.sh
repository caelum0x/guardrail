#!/usr/bin/env bash
# Bring the full Guardrail Alpha stack up or down via docker compose.
#
#   scripts/stack.sh up      # build + start agent, api, monitor, exporter,
#                            # dashboard, prometheus, grafana
#   scripts/stack.sh down    # stop and remove
#   scripts/stack.sh logs    # follow logs
#   scripts/stack.sh ps      # status
set -euo pipefail

cmd="${1:-up}"
case "$cmd" in
  up)   docker compose up -d --build ;;
  down) docker compose down ;;
  logs) docker compose logs -f ;;
  ps)   docker compose ps ;;
  *)    echo "usage: $0 {up|down|logs|ps}" >&2; exit 1 ;;
esac
