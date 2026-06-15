# CLI cookbook

Every command below runs in paper mode against deterministic mocks.

```bash
# Preflight: validate configs, policies, universe, data dir
cargo run -p guardrail-doctor

# Compile a natural-language mandate into a validated policy + hash
cargo run -p guardrail-cli -- policy compile "Trade CAKE, max drawdown 20%, kill switch 25%"

# Hash a policy file (on-chain proof fingerprint)
cargo run -p guardrail-cli -- policy hash configs/risk_policy.production.json

# Current market table via the CMC data path (mock unless --live + CMC_API_KEY)
cargo run -p guardrail-cli -- markets

# Score the eligible universe (regime + per-asset alpha)
cargo run -p guardrail-cli -- score

# Backtest the strategy (strategy vs buy-and-hold)
cargo run -p guardrail-cli -- backtest --steps 60 --preset balanced

# Walk-forward across sentiment-driven windows
cargo run -p guardrail-cli -- walk-forward --windows 6 --steps 30

# Compare all presets side by side
cargo run -p guardrail-cli -- compare --steps 60 --fear-greed 70

# Quote an AMM-style swap
cargo run -p guardrail-cli -- quote --from USDT --to CAKE --amount 500

# Agent on-chain identity + proof
cargo run -p guardrail-cli -- identity

# Render the latest run report (offline)
cargo run -p guardrail-cli -- report

# Trigger the local kill switch
cargo run -p guardrail-cli -- kill-switch --reason "manual_operator_trigger"

# Sentiment sweep / walk-forward via the research binary
cargo run -p guardrail-sim -- --steps 60 --preset aggressive
cargo run -p guardrail-sim -- --walk-forward --windows 6 --steps 30

# Audit the event log
cargo run -p guardrail-replay -- journal
cargo run -p guardrail-replay -- trades
cargo run -p guardrail-replay -- summary
cargo run -p guardrail-replay -- export-csv data/exports/events.csv
```
