#!/usr/bin/env bash
set -euo pipefail

cargo run -p guardrail-cli -- backtest --config configs/backtest.toml

