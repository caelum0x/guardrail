# API cookbook

Start the API (reads the SQLite event log written by the agent):

```bash
DATABASE_URL=sqlite://data/guardrail_alpha.db cargo run -p guardrail-api
# serves on http://localhost:8080
```

## Live state

```bash
curl localhost:8080/health
curl localhost:8080/cockpit
curl localhost:8080/portfolio
curl localhost:8080/risk
curl localhost:8080/alerts
curl localhost:8080/proof
curl localhost:8080/events
curl localhost:8080/history          # NAV equity series
curl localhost:8080/metrics          # Prometheus text
```

## Research

```bash
# Single backtest (strategy vs buy-and-hold), with preset + sentiment
curl "localhost:8080/backtest?steps=60&fear_greed=70&preset=balanced"

# Rolling walk-forward windows
curl "localhost:8080/walkforward?windows=6&steps=30&preset=aggressive"

# Sentiment sweep
curl "localhost:8080/sweep?steps=40&fear_greed=20,40,60,80&preset=conservative"
```

## Natural-language policy

```bash
curl "localhost:8080/policy/compile?mandate=Trade%20CAKE%20max%20drawdown%2020%25%20kill%20switch%2025%25%20stable%20reserve%2010%25"
```

## Reports & proof

```bash
curl localhost:8080/report                 # JSON run report
curl localhost:8080/report/markdown        # Markdown report
curl localhost:8080/export/submission.md   # submission artifact
```
