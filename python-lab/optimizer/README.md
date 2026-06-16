# guardrail-optimizer

A strategy-parameter optimizer: grid and random search over the parameters the
agent exposes (`min_score_to_enter`, `min_score_to_hold`, `max_positions`,
`rebalance_threshold_pct`, `target_stable_reserve_pct`), maximizing a chosen
objective (Calmar or Sharpe). **Pure standard library** — no numpy, no requests.

Two modes:

- **offline** (default): searches a deterministic synthetic objective surface
  whose optimum is hidden in `model.py`; the search rediscovers it. Runs with no
  API.
- **api**: ranks the named presets (`conservative`/`balanced`/`aggressive`) by
  calling the live backtest endpoint `GET /skills/{skill}/backtest?preset=`.

## Run

```bash
cd python-lab/optimizer
python3 -m optimizer.cli --offline --metric calmar
python3 -m optimizer.cli --offline --random 40 --metric sharpe
python3 -m optimizer.cli --offline --csv /tmp/grid.csv
python3 -m optimizer.cli --api --skill momentum-volatility-blend --metric calmar
```

`--top N` controls how many ranked results print; `--api-url` / `$GUARDRAIL_API`
sets the API base.
