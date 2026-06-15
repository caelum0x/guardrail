#!/usr/bin/env bash
set -euo pipefail

cargo build
if command -v pnpm >/dev/null 2>&1; then
  (cd dashboard && pnpm install)
fi
if command -v pip >/dev/null 2>&1; then
  (cd python-lab && pip install -r requirements.txt)
fi

