# Guardrail Alpha — Python Analytics Lab

Offline analytics layer for the Guardrail Alpha trading agent. It reads the
agent's SQLite event log plus its JSON run report and produces metrics, charts,
CSV exports, and Markdown reports.

**Everything is standard-library only.** `matplotlib` is the single optional
dependency: when it is installed, chart scripts emit PNGs; when it is absent they
fall back to CSV, so every script always succeeds. The packages listed in
`requirements.txt` / `pyproject.toml` are for the notebooks, not for the export
pipeline.

```bash
# No install required for the export pipeline:
python3 python-lab/scripts/verify.py        # smoke check
python3 python-lab/scripts/export_all.py     # full pipeline
```

All scripts bootstrap `sys.path` so they run from either the repository root or
from `python-lab/`. Paths below are written as run from the **repository root**.

---

## Data Sources

### 1. SQLite event log — `data/guardrail_alpha.db`

Single `events` table, one row per agent event:

| Column         | Type | Notes                                                |
| -------------- | ---- | ---------------------------------------------------- |
| `id`           | TEXT | Primary key.                                         |
| `run_id`       | TEXT | Run identifier (not null).                           |
| `timestamp`    | TEXT | ISO-8601 / sortable timestamp (not null).            |
| `event_type`   | TEXT | Event kind (not null) — see table below.             |
| `payload_json` | TEXT | JSON object string (not null), parsed to `payload`.  |

Event types and their key payload fields (one cycle, in timestamp order):

| `event_type`                        | Key payload fields                                              |
| ----------------------------------- | -------------------------------------------------------------- |
| `agent_started`                     | `agent_id`, `mode`, `policy_hash`, `wallet`                     |
| `market_snapshot_received`          | `assets`, `ts`                                                  |
| `regime_classified`                 | `regime` (e.g. `risk_on`)                                       |
| `portfolio_target_computed`         | `headline`, `orders`                                            |
| `order_proposed`                    | `from`, `to`, `amount_usd` (decimal string)                    |
| `twak_quote_received`               | `route`, `slippage_pct`                                         |
| `risk_approved` / `risk_clipped`    | `amount_usd` (final executed amount, decimal string)           |
| `twak_swap_submitted`               | `amount_usd`                                                    |
| `tx_confirmed`                      | `tx_hash`, `status`, `block` (one confirmed on-chain swap)     |
| `daily_trade_requirement_satisfied` | `trades`                                                        |
| `portfolio_reconciled`              | `nav_usd` (high-precision decimal string), `positions`         |
| `agent_report_published`            | `agent_id`, `wallet_address`, `policy_hash`, `report_hash`, `run_id`, `final_nav_usd`, `total_drawdown_pct`, `cycles`, `events`, `address_url` |

`amount_usd` / `nav_usd` are stored as high-precision **decimal strings** and are
parsed to `float` best-effort; unparseable values degrade to `0.0` / are skipped
rather than raising.

### 2. Run report — `data/run_report.json`

JSON object summarizing the run. Keys:

| Key                  | Meaning                                              |
| -------------------- | --------------------------------------------------- |
| `run_id`             | Run identifier.                                      |
| `mode`               | `paper` / `live`.                                    |
| `regime`             | Latest regime classification.                       |
| `kill_switch`        | Boolean — whether the kill switch triggered.        |
| `nav_usd`            | Current NAV (decimal string).                        |
| `starting_nav_usd`   | NAV at the start of the run.                          |
| `total_drawdown_pct` | Reported drawdown as a fraction (e.g. `0.0432`).     |
| `trades`             | Confirmed trade count.                               |
| `events`             | Event count.                                         |
| `policy_hash`        | Policy hash.                                          |
| `wallet_address`     | Agent wallet address.                                |
| `updated_ms`         | Last-update timestamp (ms).                          |
| `positions`          | Array of `{ symbol, value_usd, weight_pct }`.        |

---

## `guardrail_lab` package

Each module is stdlib-only and tolerates missing / malformed data.

| Module           | Purpose                          | Key functions                                                                 |
| ---------------- | -------------------------------- | ----------------------------------------------------------------------------- |
| `db.py`          | SQLite event-log access          | `load_events(db_path)` → `list[dict]` (parses `payload_json` to `payload`); `event_counts(events)` → `dict` by type; `database_path()` |
| `loaders.py`     | JSON run-report access           | `load_run_report(path)` → `dict \| None` (None when missing/invalid)          |
| `metrics.py`     | NAV / drawdown / counts          | `nav_series(events)` → `list[(ts, nav)]`; `drawdown_series(events)` → `list[(ts, pct)]`; `max_drawdown(values)` → `float`; `trade_count(events)` → `int` |
| `attribution.py` | Trade attribution & regimes      | `trade_attribution(events)` → `list[{symbol, count, total_amount_usd}]` (descending); `regime_timeline(events)` → `list[{timestamp, regime}]` |
| `charts.py`      | Charts (PNG) + CSV fallback      | `plot_equity_curve`, `plot_drawdown`, `plot_allocation`, `plot_attribution` (return path or `None`); `write_equity_curve_csv`, `write_allocation_csv`, `write_attribution_csv`, `write_drawdown_csv`; `PLOTTING_AVAILABLE` flag |
| `reports.py`     | Markdown reports                 | `build_daily_report(db_path, report_path)` → `str`; `build_submission_report(...)` → `str` (judge-facing) |

`trade_attribution` correlates each `tx_confirmed` swap with the most recent
preceding `order_proposed` (for the destination symbol) and the latest
`risk_approved` / `risk_clipped` decision (for the final executed amount).

---

## Scripts (`scripts/`)

Each script accepts an optional positional `[db_path]` (and, where noted,
`[report_path]`) and is safe to run when the database is missing.

| Script | Run command (from repo root) | Artifact written |
| ------ | ---------------------------- | ---------------- |
| `verify.py` | `python3 python-lab/scripts/verify.py` | None — prints a PASS/FAIL smoke-check checklist (exits non-zero on failure) |
| `export_equity_curve.py` | `python3 python-lab/scripts/export_equity_curve.py` | `data/exports/equity_curve.csv` (timestamp, nav_usd) + summary stats |
| `export_drawdown_chart.py` | `python3 python-lab/scripts/export_drawdown_chart.py` | `data/exports/drawdown.png` (matplotlib) or `data/exports/drawdown.csv` |
| `export_trade_attribution.py` | `python3 python-lab/scripts/export_trade_attribution.py` | `data/exports/trade_attribution.csv` (symbol, count, total_amount_usd) |
| `export_signal_heatmap.py` | `python3 python-lab/scripts/export_signal_heatmap.py` | `data/exports/signal_summary.csv` (regime timeline + final weights) |
| `export_charts.py` | `python3 python-lab/scripts/export_charts.py` | `data/exports/{equity_curve,allocation,attribution}.png` or `.csv` fallbacks |
| `generate_daily_report.py` | `python3 python-lab/scripts/generate_daily_report.py` | `python-lab/reports/daily/<date>.md` (or `data/exports/daily_report.md` when no run report) |
| `generate_submission_report.py` | `python3 python-lab/scripts/generate_submission_report.py` | `python-lab/reports/final_submission/submission.md` |
| `export_all.py` | `python3 python-lab/scripts/export_all.py` | Runs the full pipeline (equity, allocation, drawdown, attribution, signal summary, daily report) and prints a manifest |

### Output locations

- **`data/exports/`** — CSV/PNG export artifacts.
- **`python-lab/reports/daily/<date>.md`** — daily Markdown reports.
- **`python-lab/reports/final_submission/submission.md`** — submission report.

### PNG vs CSV

Chart scripts (`export_charts.py`, `export_drawdown_chart.py`, `export_all.py`)
write **PNG** when `matplotlib` is importable and **CSV** otherwise. The plotting
functions return `None` when plotting is unavailable or there is no data, which
is the signal the scripts use to fall back to CSV. Pure-data exports
(`export_equity_curve.py`, `export_trade_attribution.py`, `export_signal_heatmap.py`)
are always CSV.

---

## Verification

```bash
python3 python-lab/scripts/verify.py   # exit 0 on success
```

`verify.py` imports every module, checks the key functions are callable, runs
each pipeline function against an empty event log, runs the full pipeline against
`data/guardrail_alpha.db` when it exists, and writes a sample artifact to a temp
directory. When the database is absent it runs import-only checks and still
passes with a notice.
