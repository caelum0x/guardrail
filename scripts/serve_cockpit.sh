#!/usr/bin/env bash
# Serve the Guardrail web-lite cockpit (clients/web-lite) over a static HTTP
# server so it can be opened in a browser. Pure-static, no build step, offline.
#
#   scripts/serve_cockpit.sh                 # serve on :8090, API http://localhost:8080
#   scripts/serve_cockpit.sh 8095            # serve on :8095
#   GUARDRAIL_API=http://localhost:8081 scripts/serve_cockpit.sh
#
# The cockpit (clients/web-lite/index.html) reads the API base from a `?api=`
# query param, so the printed URL wires it to GUARDRAIL_API automatically.
set -euo pipefail

cd "$(dirname "$0")/.."

PORT="${1:-${COCKPIT_PORT:-8090}}"
API_BASE="${GUARDRAIL_API:-http://localhost:8080}"
WEB_LITE_DIR="clients/web-lite"

if [ ! -f "$WEB_LITE_DIR/index.html" ]; then
  echo "error: $WEB_LITE_DIR/index.html not found (run from the repo root)" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required to serve the cockpit" >&2
  exit 1
fi

COCKPIT_URL="http://localhost:${PORT}/index.html?api=${API_BASE}"

echo "============================================================"
echo "» Guardrail web-lite cockpit"
echo "============================================================"
echo "  serving:  $WEB_LITE_DIR  on  http://localhost:${PORT}"
echo "  API base: $API_BASE"
echo
echo "  Open the cockpit:"
echo "    $COCKPIT_URL"
echo
echo "  (Ctrl-C to stop)"
echo "============================================================"

exec python3 -m http.server "$PORT" --directory "$WEB_LITE_DIR"
