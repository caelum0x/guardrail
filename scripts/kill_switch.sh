#!/usr/bin/env bash
set -euo pipefail

echo "Triggering local kill switch..."
cargo run -p guardrail-cli -- kill-switch --reason "manual_operator_trigger"

