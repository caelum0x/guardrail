#!/usr/bin/env bash
# Print live submission-readiness checks from the Guardrail Alpha API.
set -euo pipefail

API_URL="${API_URL:-http://127.0.0.1:8080}"

READINESS_JSON="$(curl -fsS "${API_URL}/readiness")" node <<'NODE'
const readiness = JSON.parse(process.env.READINESS_JSON);
console.log(`status: ${readiness.status}`);
console.log(`blocking: ${readiness.blocking}`);
for (const check of readiness.checks ?? []) {
  const mark = check.status === "pass" ? "PASS" : "BLOCK";
  console.log(`${mark}\t${check.label}\t${check.detail}`);
}
NODE
