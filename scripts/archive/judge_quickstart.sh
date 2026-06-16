#!/usr/bin/env bash
# ============================================================================
# Guardrail Alpha — JUDGE QUICKSTART
# ============================================================================
# A single, idempotent, offline-safe entry point for evaluators. It:
#
#   1. Builds the workspace (cargo build; release optional via --release/env).
#   2. Runs the paper agent once to populate data/ if no run data exists.
#   3. Starts guardrail-api in the background on :8080 and waits for /health.
#   4. Serves the web-lite cockpit (clients/web-lite) on a static port, wired
#      to the API via ?api=http://localhost:8080.
#   5. Prints a clear panel of URLs (dashboard, cockpit) and curlable endpoints.
#
# Everything runs in paper mode with deterministic mocks — no API keys, no
# network access required. Background processes are cleaned up on exit.
#
# Usage:
#   scripts/judge_quickstart.sh                 # build + run + serve
#   scripts/judge_quickstart.sh --no-build      # skip cargo build
#   scripts/judge_quickstart.sh --port 8095     # cockpit static port
#   scripts/judge_quickstart.sh --api-port 8080 # API port (binary uses 8080)
#   GUARDRAIL_RELEASE=1 scripts/judge_quickstart.sh   # release build
# ============================================================================
set -euo pipefail

cd "$(dirname "$0")/.."

# --- Defaults ---------------------------------------------------------------
DO_BUILD=1
COCKPIT_PORT="${COCKPIT_PORT:-8090}"
API_PORT="${GUARDRAIL_API_PORT:-8080}"
RELEASE="${GUARDRAIL_RELEASE:-0}"
export RUST_LOG="${RUST_LOG:-error}"

DB_PATH="data/guardrail_alpha.db"
REPORT_PATH="data/run_report.json"
PAPER_CONFIG="configs/paper.toml"
WEB_LITE_DIR="clients/web-lite"

# --- Flag parsing -----------------------------------------------------------
while [ $# -gt 0 ]; do
  case "$1" in
    --no-build)  DO_BUILD=0; shift ;;
    --release)   RELEASE=1; shift ;;
    --port)      COCKPIT_PORT="${2:?--port needs a value}"; shift 2 ;;
    --port=*)    COCKPIT_PORT="${1#*=}"; shift ;;
    --api-port)  API_PORT="${2:?--api-port needs a value}"; shift 2 ;;
    --api-port=*) API_PORT="${1#*=}"; shift ;;
    -h|--help)
      sed -n '2,30p' "$0"
      exit 0 ;;
    *)
      echo "unknown flag: $1 (try --help)" >&2
      exit 1 ;;
  esac
done

API_BASE="http://localhost:${API_PORT}"

# --- Background process bookkeeping + cleanup -------------------------------
API_PID=""
COCKPIT_PID=""

cleanup() {
  local code=$?
  echo
  echo "Shutting down background processes…"
  [ -n "$COCKPIT_PID" ] && kill "$COCKPIT_PID" 2>/dev/null || true
  [ -n "$API_PID" ] && kill "$API_PID" 2>/dev/null || true
  exit "$code"
}
trap cleanup EXIT INT TERM

section() {
  echo
  echo "============================================================"
  echo "» $*"
  echo "============================================================"
}

# --- 0. Preconditions -------------------------------------------------------
if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required to serve the cockpit" >&2
  exit 1
fi
if ! command -v curl >/dev/null 2>&1; then
  echo "error: curl is required to probe the API" >&2
  exit 1
fi
if [ "$API_PORT" != "8080" ]; then
  echo "note: guardrail-api binds 0.0.0.0:8080 by default; --api-port only"
  echo "      changes the URLs this script probes/links. Use 8080 unless you"
  echo "      have separately rebuilt the API to listen elsewhere."
fi

# --- 1. Build ---------------------------------------------------------------
if [ "$DO_BUILD" -eq 1 ]; then
  if [ "$RELEASE" -eq 1 ]; then
    section "1. Build workspace (release)"
    cargo build --workspace --release --quiet
  else
    section "1. Build workspace (debug)"
    cargo build --workspace --quiet
  fi
else
  section "1. Build skipped (--no-build)"
fi

# Cargo run profile flag (reused below).
PROFILE_FLAG=""
[ "$RELEASE" -eq 1 ] && PROFILE_FLAG="--release"

# --- 2. Populate data/ via the paper agent (idempotent) ---------------------
if [ -f "$DB_PATH" ] && [ -f "$REPORT_PATH" ]; then
  section "2. Run data already present — skipping paper agent"
  echo "  found $DB_PATH"
  echo "  found $REPORT_PATH"
  echo "  (delete these to force a fresh paper run)"
else
  section "2. Populate data/ with the paper agent (offline mocks)"
  echo "  config: $PAPER_CONFIG"
  cargo run -q $PROFILE_FLAG -p guardrail-agent -- --config "$PAPER_CONFIG"
fi

# --- 3. Start guardrail-api and wait for /health ----------------------------
section "3. Start guardrail-api on :${API_PORT}"
cargo run -q $PROFILE_FLAG -p guardrail-api &
API_PID=$!
echo "  guardrail-api pid: $API_PID"
echo -n "  waiting for ${API_BASE}/health "
HEALTHY=0
for _ in $(seq 1 60); do
  if curl -fsS "${API_BASE}/health" >/dev/null 2>&1; then
    HEALTHY=1
    break
  fi
  # Bail early if the API process died.
  if ! kill -0 "$API_PID" 2>/dev/null; then
    echo
    echo "error: guardrail-api exited before becoming healthy" >&2
    exit 1
  fi
  echo -n "."
  sleep 1
done
echo
if [ "$HEALTHY" -ne 1 ]; then
  echo "error: ${API_BASE}/health did not respond in time" >&2
  exit 1
fi
echo "  API healthy at ${API_BASE}"

# --- 4. Serve the web-lite cockpit ------------------------------------------
section "4. Serve the web-lite cockpit on :${COCKPIT_PORT}"
if [ ! -f "$WEB_LITE_DIR/index.html" ]; then
  echo "error: $WEB_LITE_DIR/index.html not found" >&2
  exit 1
fi
python3 -m http.server "$COCKPIT_PORT" --directory "$WEB_LITE_DIR" >/dev/null 2>&1 &
COCKPIT_PID=$!
echo "  cockpit static server pid: $COCKPIT_PID"

COCKPIT_URL="http://localhost:${COCKPIT_PORT}/index.html?api=${API_BASE}"

# --- 5. URL + endpoint panel ------------------------------------------------
section "5. Everything is up — explore Guardrail Alpha"
cat <<EOF
  Web-lite cockpit (open this in a browser):
    $COCKPIT_URL

  Next.js dashboard (full app — start it separately):
    cd dashboard && pnpm install && pnpm dev    # then http://localhost:3000
    (point its API base at ${API_BASE} if prompted)

  API base:
    ${API_BASE}

  Curlable endpoints (try these):
    curl -s ${API_BASE}/health
    curl -s ${API_BASE}/compete
    curl -s ${API_BASE}/readiness
    curl -s ${API_BASE}/history
    curl -s ${API_BASE}/skill

  Background pids: api=${API_PID} cockpit=${COCKPIT_PID}
  Press Ctrl-C to stop both background servers and clean up.
EOF

echo
echo "Tailing — Ctrl-C to shut down…"
# Keep the script alive so the trap can clean up on Ctrl-C. Wait on the API;
# if it exits, cleanup runs via the EXIT trap.
wait "$API_PID"
