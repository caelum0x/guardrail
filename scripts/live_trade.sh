#!/usr/bin/env bash
set -euo pipefail

cargo run -p guardrail-agent -- --config configs/production.toml

