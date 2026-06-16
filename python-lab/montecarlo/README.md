# guardrail-montecarlo

Monte Carlo simulation of strategy equity outcomes — the terminal-value
distribution, max-drawdown risk, probability of loss, and probability of "ruin"
(falling below a fraction of starting capital). **Pure standard library.**

Two engines:
- **bootstrap** (default): resample with replacement from a historical return
  series (no distributional assumption). Reads the agent's NAV series from the
  event log with `--db`, else a built-in sample.
- **gbm**: geometric Brownian motion from a per-step drift `--mu` and vol `--sigma`.

## Run

```bash
cd python-lab/montecarlo
python3 -m montecarlo.cli --demo
python3 -m montecarlo.cli --gbm --mu 0.0005 --sigma 0.02 --paths 5000 --horizon 180
python3 -m montecarlo.cli --bootstrap --db ../../data/guardrail_alpha.db --paths 5000
```

Output is JSON: terminal p5/p50/p95/mean, return percentiles, `max_drawdown_p95`,
`prob_loss`, and `prob_ruin` at the `--ruin` threshold (default 0.5 of start).
