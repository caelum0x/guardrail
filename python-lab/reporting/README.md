# guardrail-reporting

A self-contained HTML (and plain-text) report generator for the Guardrail
agent. It reads the agent's SQLite event log and JSON run report, computes
summary risk metrics from the NAV series, and renders a clean report with **no
external assets and no third-party dependencies** — standard library only.

## What it reads

- **`data/guardrail_alpha.db`** — a single `events` table with columns
  `(id, run_id, timestamp, event_type, payload_json)`. The generator extracts:
  - **event counts** by `event_type`
  - the **NAV series** from `portfolio_reconciled` events (`payload.nav_usd`)
  - **confirmed trades** from `tx_confirmed` events (`tx_hash`, `competition_tx`,
    `block`, `status`)
  The database is opened **read-only** so the agent's log is never mutated.
- **`data/run_report.json`** — flat run summary (`run_id`, `mode`, `regime`,
  `kill_switch`, `nav_usd`, `starting_nav_usd`, `positions`, `trades`, ...).
  Optional: if absent the report still renders from the event log alone.

## Metrics (computed inline, stdlib only)

All math uses `decimal.Decimal` for exactness — see `reporting/metrics.py`.

- **Total return** — `(last NAV / first NAV) − 1`.
- **Max drawdown** — largest peak-to-trough decline over the NAV series, as a
  non-negative ratio.
- **Simple Sharpe** — `mean(period return) / stdev(period return)` over the
  simple returns between successive NAV observations. Risk-free rate is assumed
  zero and the ratio is reported **un-annualised**.
- **Volatility** — sample standard deviation (n−1) of the period returns.

## Usage

```bash
# from python-lab/reporting/
python -m reporting.cli --db ../../data/guardrail_alpha.db --out report.html

# text summary to stdout
python -m reporting.cli --db ../../data/guardrail_alpha.db --format text

# explicit run report, single run, custom title
python -m reporting.cli \
  --db ../../data/guardrail_alpha.db \
  --report ../../data/run_report.json \
  --run-id run_0485d42cebfd45a79219ec8bc2219f09 \
  --title "Guardrail Alpha" \
  --out report.html
```

### CLI options

| Option        | Description                                                        |
|---------------|--------------------------------------------------------------------|
| `--db`        | Path to the SQLite event log (required).                           |
| `--report`    | Path to `run_report.json`. Defaults to a sibling of `--db`.        |
| `--no-report` | Ignore any run report, even one beside `--db`.                     |
| `--run-id`    | Restrict the event log to one `run_id`.                            |
| `--format`    | `html` (default) or `text`.                                        |
| `--out`       | Output file path. Defaults to stdout.                              |
| `--title`     | Document title for HTML output.                                    |

An installed entry point `guardrail-report` mirrors `python -m reporting.cli`.

## Layout

```
reporting/
├── pyproject.toml          # no dependencies
├── README.md
└── reporting/
    ├── __init__.py
    ├── data.py             # sqlite + json readers (read-only, validated)
    ├── metrics.py          # inline risk metrics (Decimal, stdlib)
    ├── format.py           # shared value formatting helpers
    ├── html.py             # self-contained HTML template + inline SVG chart
    ├── text.py             # plain-text summary renderer
    └── cli.py              # python -m reporting.cli
```

## Notes

- The HTML report embeds an inline SVG NAV chart and inline CSS; there are no
  `http`/`https` references and no JavaScript, so it opens correctly offline.
- All user-derived values are HTML-escaped before rendering.
- Pure standard library: `sqlite3`, `json`, `decimal`, `html`, `argparse`.
