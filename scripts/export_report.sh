#!/usr/bin/env bash
set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:8080}"
OUT_DIR="${OUT_DIR:-data/exports}"
REPORT_JSON="${GUARDRAIL_REPORT:-data/run_report.json}"

mkdir -p "$OUT_DIR"

if curl -fsS "$API_URL/export/submission.md" -o "$OUT_DIR/submission.md"; then
  curl -fsS "$API_URL/report" -o "$OUT_DIR/run_report.response.json" || true
  echo "Exported submission markdown from API to $OUT_DIR/submission.md"
  exit 0
fi

if [[ ! -f "$REPORT_JSON" ]]; then
  echo "No API export available and local report not found: $REPORT_JSON" >&2
  exit 1
fi

cp "$REPORT_JSON" "$OUT_DIR/run_report.json"
{
  echo "# Guardrail Alpha Submission"
  echo
  echo '```json'
  cat "$REPORT_JSON"
  echo
  echo '```'
} > "$OUT_DIR/submission.md"

echo "Exported local report artifacts to $OUT_DIR"
