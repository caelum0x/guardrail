#!/usr/bin/env bash
# Guardrail Alpha — full end-to-end demo.
#
# Exercises the entire pipeline in one command:
#   doctor (preflight) -> policy compile (NL->policy) -> paper agent run
#   (SQLite events + run report) -> replay (audit) -> exporter (metrics)
#   -> backtest / walk-forward / sweep -> markets -> identity -> submission report.
#
# Reproducible and offline: paper mode uses deterministic mocks (no keys/network).
set -euo pipefail

cd "$(dirname "$0")/.."
export RUST_LOG="${RUST_LOG:-error}"
DB="${DATABASE_URL:-sqlite://data/guardrail_alpha.db}"
CYCLES="${GUARDRAIL_CYCLES:-3}"

bin() { echo; echo "============================================================"; echo "» $*"; echo "============================================================"; }

echo "Building workspace (release-quiet)…"
cargo build --workspace --quiet

bin "1. Preflight readiness (guardrail-doctor)"
cargo run -q -p guardrail-doctor

bin "2. Compile a natural-language mandate into a validated policy"
cargo run -q -p guardrail-cli -- policy compile \
  "Trade CAKE and WBNB on BSC. Max drawdown 20%, daily loss 6%, max position 15%, \
   stable reserve 12%, slippage 0.5%, kill switch 24%, at least 1 trade per day, no leverage."

bin "3. Run the paper trading agent ($CYCLES cycles)"
rm -f data/guardrail_alpha.db data/run_report.json
GUARDRAIL_CYCLES="$CYCLES" cargo run -q -p guardrail-agent -- --config configs/paper.toml

bin "4. Audit the event log (guardrail-replay)"
cargo run -q -p guardrail-replay -- summary
cargo run -q -p guardrail-replay -- trades

bin "5. Prometheus metrics from the run (guardrail-exporter)"
EXPORTER_ADDR="127.0.0.1:9109" cargo run -q -p guardrail-exporter &
EXP_PID=$!
sleep 1
curl -fsS "http://127.0.0.1:9109/metrics" | grep -E "guardrail_(nav_usd|trades_total|total_drawdown_pct|positions|kill_switch)" || true
kill "$EXP_PID" 2>/dev/null || true

bin "6. Current market table (guardrail-cli markets)"
cargo run -q -p guardrail-cli -- markets

bin "7. Backtest the strategy (guardrail-cli backtest)"
cargo run -q -p guardrail-cli -- backtest --steps 60

bin "8. Walk-forward across sentiment regimes (guardrail-cli walk-forward)"
cargo run -q -p guardrail-cli -- walk-forward --windows 6 --steps 30

bin "9. Sentiment sweep (guardrail-sim)"
cargo run -q -p guardrail-sim -- --steps 60

bin "10. Agent on-chain identity + proof (guardrail-cli identity)"
cargo run -q -p guardrail-cli -- identity

bin "11. Analytics + submission report (python-lab)"
if command -v python3 >/dev/null 2>&1; then
  python3 python-lab/scripts/export_all.py || true
  python3 python-lab/scripts/generate_submission_report.py || true
else
  echo "python3 not found; skipping analytics"
fi

echo
echo "============================================================"
echo "Demo complete. Start the API + dashboard to explore:"
echo "  cargo run -p guardrail-api          # http://localhost:8080"
echo "  (cd dashboard && pnpm dev)          # http://localhost:3000"
echo "============================================================"
