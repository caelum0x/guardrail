# risk-analytics

A read-only service that computes portfolio **risk and performance metrics** over
the Guardrail agent's NAV curve: total/annualized return, annualized volatility,
max drawdown, Sharpe, Sortino, Calmar, historical & parametric VaR, and a
pairwise correlation matrix.

The metrics engine (`risk_analytics.metrics`) and the CLI are **pure
standard-library Python** — no numpy, no third-party deps — so they run anywhere.
FastAPI/uvicorn are only needed to serve the HTTP API.

## CLI (no install needed)

```bash
cd services/risk-analytics
python3 -m risk_analytics.cli demo                 # built-in sample curve
python3 -m risk_analytics.cli metrics equity.json  # JSON array of NAV points
python3 -m risk_analytics.cli live --db ../../data/guardrail_alpha.db
```

## HTTP API

```bash
pip install -e '.[serve]'
uvicorn risk_analytics.api:app --port 8092
```

| Route | Description |
|---|---|
| `GET /health` | Liveness. |
| `POST /metrics` | Body `{equity: number[], periods_per_year?}` → full metric suite. |
| `POST /correlation` | Body `{series: {name: returns[]}}` → correlation matrix. |
| `GET /metrics/live` | Metrics over the NAV series in the agent's event log. |

## Notes

- `periods_per_year` defaults to 365 (crypto trades daily); override per your
  sampling cadence.
- VaR is reported as a positive loss fraction at the given confidence (default 95%).
- Correlation uses Pearson over equal-length return series.
