# Guardrail Alpha — Analysis Notebooks

Six Jupyter notebooks that load the agent's **event log** (`data/guardrail_alpha.db`)
and **run report** (`data/run_report.json`) and render the research / submission
analyses. They reuse the `guardrail_lab` package — they do **not** re-implement
any analytics.

| Notebook | What it shows |
|----------|---------------|
| `01_universe_filtering.ipynb` | The 20-token BSC universe from `configs/eligible_assets.bsc.json`, with category breakdown. |
| `02_signal_research.ipynb` | Regime classifications, time-in-regime, exposure multipliers, and per-asset scores (`regime_analysis`). |
| `03_backtest_review.ipynb` | Backtest / experiment records from `data/experiments/` and `data/backtests/` compared side by side (`experiments`). |
| `04_live_pnl_analysis.ipynb` | NAV curve + drawdown from the event log (`metrics`, `drawdown`). |
| `05_trade_attribution.ipynb` | Per-asset attribution of confirmed swaps (`attribution`). |
| `06_submission_charts.ipynb` | Headline submission tables: identity, performance, regime time, drawdown, attribution. |

## Offline-safe by design

These notebooks are **100% offline-safe** and run with only the Python standard
library plus `guardrail_lab`:

- **No network access** — they read local SQLite + JSON only.
- **No heavy plotting deps** — there is **no `matplotlib`/`plotly` import**.
  Tables are printed as aligned text and curves as ASCII sparklines, so the
  notebooks run even on a bare interpreter. (If you later install the optional
  plotting stack, `guardrail_lab.charts` / `guardrail_lab.submission` build
  richer HTML/PNG output from the same functions.)
- **Graceful degradation** — when `data/` is empty (no run yet), every cell
  prints a "run the agent / seed a demo first" hint instead of raising.

Each notebook starts with a **bootstrap cell** that walks up from the current
working directory to the repo root (the folder containing
`python-lab/guardrail_lab`), puts `python-lab` on `sys.path`, and resolves the
data/config paths. As a result the notebooks work whether you launch Jupyter
from the repo root or from `python-lab/notebooks/`. They prefer a real run and
fall back to the seeded demo artifacts (`data/demo_*`) when present.

## How to run

### Jupyter (interactive)

From the repo root:

```bash
cd python-lab
pip install -r requirements.txt        # installs jupyter (one-time)
jupyter lab notebooks/                  # or: jupyter notebook notebooks/
```

Open any notebook and **Run All**. The first (bootstrap) cell prints the
resolved repo root, event-log path, and run-report path.

### Headless (no Jupyter needed)

Because the notebooks only use the stdlib + `guardrail_lab`, you can execute a
notebook's code cells directly without Jupyter:

```bash
# From the repo root:
python3 -c "import json; nb=json.load(open('python-lab/notebooks/04_live_pnl_analysis.ipynb')); \
src=''.join(''.join(c['source']) for c in nb['cells'] if c['cell_type']=='code'); exec(src)"
```

Validate all notebooks are well-formed JSON:

```bash
python3 -c "import json,glob; [json.load(open(f)) for f in glob.glob('python-lab/notebooks/*.ipynb')]; \
print('nb json ok', len(glob.glob('python-lab/notebooks/*.ipynb')))"
```

## Getting data to analyze

If `data/` is empty, populate it first (then re-run any notebook):

```bash
# Seed a synthetic demo run (offline):
cd python-lab
python3 -m guardrail_lab.seed

# …or run the real agent (paper/live) which writes data/guardrail_alpha.db
# and data/run_report.json.
```

## Regenerating the notebooks

The notebook JSON is generated from a single authoritative script so it is easy
to keep consistent. To rebuild all six `.ipynb` files:

```bash
python3 python-lab/notebooks/_build_notebooks.py
```
