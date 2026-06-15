"""Generator for the Guardrail Alpha analysis notebooks.

Run with ``python3 python-lab/notebooks/_build_notebooks.py`` (or from any cwd)
to (re)write the six ``.ipynb`` files in this directory as valid nbformat-4
JSON. The notebooks themselves only depend on the Python standard library and
``guardrail_lab`` so they run with the base analytics stack -- no matplotlib or
nbformat required at run time. Tables are rendered as printed text / Markdown.

This generator keeps the notebook JSON authoritative and easy to regenerate;
it is not imported by the notebooks at run time.
"""

from __future__ import annotations

import json
from pathlib import Path

NB_DIR = Path(__file__).resolve().parent


def md(*lines: str) -> dict:
    """Build a Markdown cell. ``source`` is a list of strings (newline-joined)."""
    text = "\n".join(lines)
    return {
        "cell_type": "markdown",
        "metadata": {},
        "source": _as_source(text),
    }


def code(text: str) -> dict:
    """Build a code cell from a (possibly multi-line) source string."""
    return {
        "cell_type": "code",
        "execution_count": None,
        "metadata": {},
        "outputs": [],
        "source": _as_source(text.strip("\n")),
    }


def _as_source(text: str) -> list[str]:
    """Split text into an nbformat ``source`` array (newline-terminated lines).

    Every line -- including the last -- keeps a trailing newline so that
    concatenating the source of multiple code cells (e.g. for headless
    extraction via ``''.join(...)``) never fuses the final line of one cell to
    the first line of the next.
    """
    return [line + "\n" for line in text.split("\n")]


def notebook(cells: list[dict]) -> dict:
    """Wrap cells in a minimal, valid nbformat-4 notebook document."""
    return {
        "cells": cells,
        "metadata": {
            "kernelspec": {
                "display_name": "Python 3",
                "language": "python",
                "name": "python3",
            },
            "language_info": {"name": "python", "version": "3.11"},
        },
        "nbformat": 4,
        "nbformat_minor": 5,
    }


# ---------------------------------------------------------------------------
# Shared bootstrap cell.
#
# Notebooks live in ``python-lab/notebooks``. This cell walks up to the repo
# root (the directory that contains ``python-lab/guardrail_lab``), puts
# ``python-lab`` on ``sys.path`` so ``import guardrail_lab`` works, and exposes
# convenient resolved paths to the event log, run report, experiments, configs,
# and backtests. Everything is offline-safe: missing files are reported, not
# fatal.
# ---------------------------------------------------------------------------
BOOTSTRAP = '''
# --- Guardrail Alpha notebook bootstrap (offline-safe) ---
import sys
from pathlib import Path


def _find_repo_root(start: Path) -> Path:
    """Return the first ancestor that contains python-lab/guardrail_lab."""
    for candidate in [start, *start.parents]:
        if (candidate / "python-lab" / "guardrail_lab").is_dir():
            return candidate
    return start


# In a notebook __file__ is undefined, so anchor on the current working dir.
REPO_ROOT = _find_repo_root(Path.cwd())
LAB_PATH = REPO_ROOT / "python-lab"
if str(LAB_PATH) not in sys.path:
    sys.path.insert(0, str(LAB_PATH))


def _first_existing(*candidates: Path):
    """Return the first path that exists, else None."""
    for path in candidates:
        if path.exists():
            return path
    return None


DATA_DIR = REPO_ROOT / "data"
CONFIG_DIR = REPO_ROOT / "configs"

# Prefer a real run; fall back to the seeded demo artifacts if present.
DB_PATH = _first_existing(
    DATA_DIR / "guardrail_alpha.db",
    DATA_DIR / "demo_guardrail_alpha.db",
)
RUN_REPORT_PATH = _first_existing(
    DATA_DIR / "run_report.json",
    DATA_DIR / "demo_run_report.json",
)
EXPERIMENTS_DIR = DATA_DIR / "experiments"
BACKTESTS_DIR = DATA_DIR / "backtests"
ELIGIBLE_ASSETS_PATH = CONFIG_DIR / "eligible_assets.bsc.json"
ASSET_CATEGORIES_PATH = CONFIG_DIR / "asset_categories.json"

NO_DATA_HINT = (
    "No data found under data/. Run the agent (paper/live) or seed a demo run "
    "first, e.g.  python3 -m guardrail_lab.seed  (from python-lab/), then "
    "re-run this notebook. The notebook is offline-safe and will not raise."
)

print("repo root :", REPO_ROOT)
print("event log :", DB_PATH if DB_PATH else "(none yet)")
print("run report:", RUN_REPORT_PATH if RUN_REPORT_PATH else "(none yet)")
'''.strip()


def bootstrap_cell() -> dict:
    return code(BOOTSTRAP)


# Small text-table helper reused inside notebooks (defined inline per notebook
# so each notebook is self-contained and import-clean).
TABLE_HELPER = '''
def render_table(rows, columns, title=None):
    """Print an aligned text table. rows: list[dict]; columns: list[(key,label)]."""
    if title:
        print(title)
        print("=" * len(title))
    if not rows:
        print("(no rows)")
        return
    labels = [label for _, label in columns]
    keys = [key for key, _ in columns]
    cells = [[("" if r.get(k) is None else str(r.get(k))) for k in keys] for r in rows]
    widths = [
        max(len(labels[i]), *(len(row[i]) for row in cells)) for i in range(len(keys))
    ]
    header = "  ".join(labels[i].ljust(widths[i]) for i in range(len(keys)))
    print(header)
    print("  ".join("-" * widths[i] for i in range(len(keys))))
    for row in cells:
        print("  ".join(row[i].ljust(widths[i]) for i in range(len(keys))))
'''.strip()


# ===========================================================================
# 01 - Universe filtering
# ===========================================================================
def build_01() -> dict:
    cells = [
        md(
            "# 01 - Universe Filtering",
            "",
            "The Guardrail Alpha agent only trades a curated BSC universe. This",
            "notebook loads `configs/eligible_assets.bsc.json`, shows the full",
            "20-token universe, and breaks it down by category.",
            "",
            "**Offline-safe:** if the config is missing the notebook prints a",
            "hint instead of raising.",
        ),
        bootstrap_cell(),
        code(TABLE_HELPER),
        md("## Load the eligible-asset universe"),
        code(
            '''
import json

universe = []
if ELIGIBLE_ASSETS_PATH.exists():
    try:
        universe = json.loads(ELIGIBLE_ASSETS_PATH.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError) as exc:
        print("Could not parse eligible_assets.bsc.json:", exc)
        universe = []

if not universe:
    print(NO_DATA_HINT)
else:
    enabled = [a for a in universe if a.get("enabled")]
    print(f"Universe size: {len(universe)} tokens "
          f"({len(enabled)} enabled).")
'''
        ),
        md("## Full universe table"),
        code(
            '''
if universe:
    rows = [
        {
            "symbol": a.get("symbol"),
            "category": a.get("category"),
            "cmc_id": a.get("cmc_id"),
            "enabled": a.get("enabled"),
            "min_liq_usd": a.get("min_liquidity_usd"),
            "min_vol24h_usd": a.get("min_volume_24h_usd"),
        }
        for a in sorted(universe, key=lambda x: (x.get("category", ""), x.get("symbol", "")))
    ]
    render_table(
        rows,
        [
            ("symbol", "SYMBOL"),
            ("category", "CATEGORY"),
            ("cmc_id", "CMC_ID"),
            ("enabled", "ENABLED"),
            ("min_liq_usd", "MIN_LIQ_USD"),
            ("min_vol24h_usd", "MIN_VOL24H_USD"),
        ],
        title="Eligible BSC universe",
    )
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Category breakdown"),
        code(
            '''
if universe:
    by_cat = {}
    for a in universe:
        cat = a.get("category", "unknown")
        by_cat.setdefault(cat, []).append(a.get("symbol"))

    cat_rows = [
        {"category": cat, "count": len(syms), "symbols": ", ".join(sorted(s for s in syms if s))}
        for cat, syms in sorted(by_cat.items(), key=lambda kv: (-len(kv[1]), kv[0]))
    ]
    render_table(
        cat_rows,
        [("category", "CATEGORY"), ("count", "COUNT"), ("symbols", "SYMBOLS")],
        title="Universe by category",
    )
else:
    print(NO_DATA_HINT)
'''
        ),
        md(
            "## Notes",
            "",
            "* `min_liquidity_usd` / `min_volume_24h_usd` are the pre-trade",
            "  eligibility gates the agent enforces before an asset can be",
            "  scored or traded.",
            "* Stable assets (e.g. USDT/USDC) are the settlement legs; the agent",
            "  rotates risk capital across the non-stable categories.",
        ),
    ]
    return notebook(cells)


# ===========================================================================
# 02 - Signal research
# ===========================================================================
def build_02() -> dict:
    cells = [
        md(
            "# 02 - Signal Research",
            "",
            "Regime classifications and per-asset scores over the event log.",
            "Uses `guardrail_lab.regime_analysis` for the transition matrix,",
            "time-in-regime, and per-regime exposure, plus the raw",
            "`asset_scored` events for the signal table.",
            "",
            "**Offline-safe:** an empty event log prints a hint instead of",
            "raising.",
        ),
        bootstrap_cell(),
        code(TABLE_HELPER),
        md("## Load events"),
        code(
            '''
from guardrail_lab.db import load_events, event_counts

events = load_events(str(DB_PATH)) if DB_PATH else []
if not events:
    print(NO_DATA_HINT)
else:
    print(f"Loaded {len(events)} events.")
    counts = event_counts(events)
    render_table(
        [{"event_type": k, "count": v} for k, v in counts.items()],
        [("event_type", "EVENT_TYPE"), ("count", "COUNT")],
        title="Event-type counts",
    )
'''
        ),
        md("## Regime analytics"),
        code(
            '''
from guardrail_lab.regime_analysis import analyze_regimes

if events:
    analysis = analyze_regimes(events)

    print(f"Regimes observed: {analysis.transitions.regimes or '(none)'}")
    print(f"Total transitions: {analysis.transitions.total_transitions}\\n")

    render_table(
        [
            {
                "regime": t.regime,
                "classifications": t.classifications,
                "fraction": f"{t.fraction:.2%}",
                "seconds": f"{t.seconds:.1f}",
            }
            for t in analysis.time_in_regime
        ],
        [
            ("regime", "REGIME"),
            ("classifications", "COUNT"),
            ("fraction", "SHARE"),
            ("seconds", "SECONDS"),
        ],
        title="Time in regime",
    )
    print()
    render_table(
        [
            {
                "regime": e.regime,
                "orders": e.order_count,
                "avg_order_usd": f"{e.avg_order_usd:,.2f}",
                "exposure_x": f"{e.exposure_multiplier:.3f}",
            }
            for e in analysis.exposure
        ],
        [
            ("regime", "REGIME"),
            ("orders", "ORDERS"),
            ("avg_order_usd", "AVG_ORDER_USD"),
            ("exposure_x", "EXPOSURE_x"),
        ],
        title="Exposure multiplier by regime",
    )
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Asset scores"),
        code(
            '''
# asset_scored events: {"symbol": ..., "score": ...}
def _to_float(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None

if events:
    score_rows = []
    for ev in events:
        if ev.get("event_type") != "asset_scored":
            continue
        payload = ev.get("payload") or {}
        score_rows.append(
            {
                "timestamp": ev.get("timestamp", ""),
                "symbol": payload.get("symbol", "?"),
                "score": _to_float(payload.get("score")),
            }
        )

    # Aggregate: mean score + count per symbol, ranked.
    agg = {}
    for r in score_rows:
        sym = r["symbol"]
        slot = agg.setdefault(sym, {"symbol": sym, "n": 0, "sum": 0.0, "last": None})
        slot["n"] += 1
        if r["score"] is not None:
            slot["sum"] += r["score"]
            slot["last"] = r["score"]

    ranked = sorted(
        (
            {
                "symbol": s["symbol"],
                "times_scored": s["n"],
                "mean_score": f"{(s['sum'] / s['n']):.3f}" if s["n"] else "n/a",
                "last_score": f"{s['last']:.3f}" if s["last"] is not None else "n/a",
            }
            for s in agg.values()
        ),
        key=lambda x: x["mean_score"],
        reverse=True,
    )

    if ranked:
        render_table(
            ranked,
            [
                ("symbol", "SYMBOL"),
                ("times_scored", "TIMES_SCORED"),
                ("mean_score", "MEAN_SCORE"),
                ("last_score", "LAST_SCORE"),
            ],
            title="Asset scores (ranked by mean score)",
        )
    else:
        print("No asset_scored events in the log yet.")
else:
    print(NO_DATA_HINT)
'''
        ),
        md(
            "## Notes",
            "",
            "* The exposure multiplier (`EXPOSURE_x`) shows how much larger or",
            "  smaller proposed orders were during each regime versus the",
            "  overall mean order size (1.0 = baseline).",
            "* Higher mean scores indicate assets the signal stack favoured most",
            "  consistently across cycles.",
        ),
    ]
    return notebook(cells)


# ===========================================================================
# 03 - Backtest review
# ===========================================================================
def build_03() -> dict:
    cells = [
        md(
            "# 03 - Backtest Review",
            "",
            "Loads backtest / experiment records written by the Rust CLI into",
            "`data/experiments/` and shows their metrics side by side using",
            "`guardrail_lab.experiments`.",
            "",
            "**Offline-safe:** if no records exist the notebook prints a hint.",
        ),
        bootstrap_cell(),
        code(TABLE_HELPER),
        md("## Load experiment / backtest records"),
        code(
            '''
from guardrail_lab.experiments import load_experiments, compare_table

# Look in both the experiments dir and the backtests dir.
experiments = load_experiments(str(EXPERIMENTS_DIR))
backtests = load_experiments(str(BACKTESTS_DIR))
records = experiments + backtests

if not records:
    print(NO_DATA_HINT)
    print("\\nLooked in:")
    print(" -", EXPERIMENTS_DIR)
    print(" -", BACKTESTS_DIR)
else:
    print(f"Loaded {len(records)} record(s) "
          f"({len(experiments)} experiments, {len(backtests)} backtests).")
'''
        ),
        md("## Metrics comparison"),
        code(
            '''
def _fmt(value, suffix=""):
    if value is None:
        return "n/a"
    return f"{value:,.3f}{suffix}" if isinstance(value, float) else f"{value}{suffix}"

if records:
    rows = compare_table(records)
    table = [
        {
            "tag": r["tag"],
            "preset": r["preset"],
            "steps": r["steps"],
            "fng": r["fear_greed"],
            "return": _fmt(r["total_return_pct"], "%"),
            "max_dd": _fmt(r["max_drawdown_pct"], "%"),
            "trades": r["trade_count"],
            "win%": _fmt(r["win_rate_pct"], "%"),
            "pf": _fmt(r["profit_factor"]),
            "calmar": _fmt(r["calmar_ratio"]),
            "excess": _fmt(r["excess_return_pct"], "%"),
            "final_nav": _fmt(r["final_nav_usd"]),
        }
        for r in rows
    ]
    render_table(
        table,
        [
            ("tag", "TAG"),
            ("preset", "PRESET"),
            ("steps", "STEPS"),
            ("fng", "F&G"),
            ("return", "RETURN"),
            ("max_dd", "MAX_DD"),
            ("trades", "TRADES"),
            ("win%", "WIN"),
            ("pf", "PROFIT_FACTOR"),
            ("calmar", "CALMAR"),
            ("excess", "EXCESS"),
            ("final_nav", "FINAL_NAV"),
        ],
        title="Backtest / experiment comparison",
    )
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Best / worst by total return"),
        code(
            '''
if records:
    scored = [
        r for r in compare_table(records) if r["total_return_pct"] is not None
    ]
    if scored:
        best = max(scored, key=lambda r: r["total_return_pct"])
        worst = min(scored, key=lambda r: r["total_return_pct"])
        print(f"Best total return : {best['tag']} "
              f"({best['total_return_pct']:.3f}%, preset={best['preset']})")
        print(f"Worst total return: {worst['tag']} "
              f"({worst['total_return_pct']:.3f}%, preset={worst['preset']})")
    else:
        print("No numeric total_return_pct values to rank.")
else:
    print(NO_DATA_HINT)
'''
        ),
        md(
            "## Notes",
            "",
            "* `EXCESS` is return over the buy-and-hold benchmark; a negative",
            "  value means the strategy trailed simply holding the basket over",
            "  that window.",
            "* `CALMAR` = return / max drawdown; high values reflect very small",
            "  drawdowns in short synthetic runs.",
        ),
    ]
    return notebook(cells)


# ===========================================================================
# 04 - Live PnL analysis
# ===========================================================================
def build_04() -> dict:
    cells = [
        md(
            "# 04 - Live PnL Analysis",
            "",
            "NAV curve and drawdown from the event log. Uses",
            "`guardrail_lab.metrics.nav_series` for the NAV curve and",
            "`guardrail_lab.drawdown` for the underwater series and worst",
            "drawdown episodes. Charts are rendered as text sparklines so the",
            "notebook runs with the base stack (no matplotlib required).",
            "",
            "**Offline-safe:** an empty NAV history prints a hint.",
        ),
        bootstrap_cell(),
        code(TABLE_HELPER),
        md("## NAV curve"),
        code(
            '''
from guardrail_lab.db import load_events
from guardrail_lab.metrics import nav_series, max_drawdown, trade_count

events = load_events(str(DB_PATH)) if DB_PATH else []
curve = nav_series(events)

if not curve:
    print(NO_DATA_HINT)
else:
    navs = [nav for _, nav in curve]
    start, end = navs[0], navs[-1]
    ret_pct = (end - start) / start * 100.0 if start else 0.0
    print(f"NAV points     : {len(curve)}")
    print(f"Starting NAV   : {start:,.2f}")
    print(f"Latest NAV     : {end:,.2f}")
    print(f"Total return   : {ret_pct:+.3f}%")
    print(f"Peak NAV       : {max(navs):,.2f}")
    print(f"Trough NAV     : {min(navs):,.2f}")
    print(f"Max drawdown   : {max_drawdown(navs) * 100.0:.3f}%")
    print(f"Confirmed trades: {trade_count(events)}")
'''
        ),
        md("## NAV sparkline (text)"),
        code(
            '''
def sparkline(values):
    """ASCII sparkline using block characters; safe for any terminal."""
    if not values:
        return "(empty)"
    blocks = "▁▂▃▄▅▆▇█"
    lo, hi = min(values), max(values)
    span = (hi - lo) or 1.0
    return "".join(blocks[min(7, int((v - lo) / span * 7))] for v in values)

if curve:
    navs = [nav for _, nav in curve]
    print("NAV:", sparkline(navs))
    render_table(
        [
            {"timestamp": ts, "nav_usd": f"{nav:,.2f}"}
            for ts, nav in curve
        ],
        [("timestamp", "TIMESTAMP"), ("nav_usd", "NAV_USD")],
        title="NAV series",
    )
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Drawdown analysis"),
        code(
            '''
from guardrail_lab.drawdown import analyze_drawdown_from_events

if events and curve:
    report = analyze_drawdown_from_events(events, top_n=5)
    print(f"Max drawdown   : {report.max_drawdown_pct:.4f}%")
    print(f"Peak           : {report.peak_timestamp or 'n/a'}")
    print(f"Trough         : {report.trough_timestamp or 'n/a'}")
    dd_secs = report.max_drawdown_seconds
    rec_secs = report.max_recovery_seconds
    print(f"Drawdown dur.  : {dd_secs if dd_secs is not None else 'n/a'} s")
    print(f"Recovery dur.  : {rec_secs if rec_secs is not None else 'n/a (unrecovered)'} s\\n")

    underwater = [p.drawdown_pct for p in report.points]
    print("Underwater:", sparkline([-x for x in underwater]) if underwater else "(empty)")
    print()

    if report.episodes:
        render_table(
            [
                {
                    "depth_pct": f"{ep.depth_pct:.4f}%",
                    "peak": ep.peak_timestamp,
                    "trough": ep.trough_timestamp,
                    "recovered": ep.recovered,
                }
                for ep in report.episodes
            ],
            [
                ("depth_pct", "DEPTH"),
                ("peak", "PEAK_TS"),
                ("trough", "TROUGH_TS"),
                ("recovered", "RECOVERED"),
            ],
            title="Worst drawdown episodes",
        )
    else:
        print("No drawdown episodes (NAV never declined below a prior peak).")
else:
    print(NO_DATA_HINT)
'''
        ),
        md(
            "## Notes",
            "",
            "* NAV is taken from `portfolio_reconciled` events; each one carries",
            "  a high-precision `nav_usd`.",
            "* The underwater sparkline is inverted (taller = deeper drawdown).",
        ),
    ]
    return notebook(cells)


# ===========================================================================
# 05 - Trade attribution
# ===========================================================================
def build_05() -> dict:
    cells = [
        md(
            "# 05 - Trade Attribution",
            "",
            "Per-asset attribution of confirmed swaps using",
            "`guardrail_lab.attribution`. Each `tx_confirmed` is correlated with",
            "its preceding `order_proposed` (destination symbol) and risk",
            "decision (final executed amount).",
            "",
            "**Offline-safe:** an empty log prints a hint.",
        ),
        bootstrap_cell(),
        code(TABLE_HELPER),
        md("## Per-asset attribution"),
        code(
            '''
from guardrail_lab.db import load_events
from guardrail_lab.attribution import trade_attribution, regime_timeline

events = load_events(str(DB_PATH)) if DB_PATH else []
attribution = trade_attribution(events)

if not attribution:
    print(NO_DATA_HINT)
else:
    total_usd = sum(a["total_amount_usd"] for a in attribution)
    total_trades = sum(a["count"] for a in attribution)
    print(f"Confirmed swaps: {total_trades}  |  Total notional: {total_usd:,.2f} USD\\n")
    render_table(
        [
            {
                "symbol": a["symbol"],
                "trades": a["count"],
                "notional_usd": f"{a['total_amount_usd']:,.2f}",
                "share": f"{(a['total_amount_usd'] / total_usd * 100.0):.1f}%" if total_usd else "n/a",
            }
            for a in attribution
        ],
        [
            ("symbol", "SYMBOL"),
            ("trades", "TRADES"),
            ("notional_usd", "NOTIONAL_USD"),
            ("share", "SHARE"),
        ],
        title="Per-asset trade attribution",
    )
'''
        ),
        md("## Notional sparkline by asset"),
        code(
            '''
def bar(value, max_value, width=30):
    if not max_value:
        return ""
    filled = int(round(value / max_value * width))
    return "█" * filled + "·" * (width - filled)

if attribution:
    max_usd = max(a["total_amount_usd"] for a in attribution)
    for a in attribution:
        print(f"{a['symbol']:>6}  {bar(a['total_amount_usd'], max_usd)}  "
              f"{a['total_amount_usd']:,.2f}")
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Regime timeline (context)"),
        code(
            '''
if events:
    timeline = regime_timeline(events)
    if timeline:
        render_table(
            timeline,
            [("timestamp", "TIMESTAMP"), ("regime", "REGIME")],
            title="Regime classifications over time",
        )
    else:
        print("No regime_classified events in the log yet.")
else:
    print(NO_DATA_HINT)
'''
        ),
        md(
            "## Notes",
            "",
            "* Notional uses the risk-adjusted amount when a `risk_approved` /",
            "  `risk_clipped` decision is present, otherwise the proposed order",
            "  amount.",
            "* Unknown destinations are bucketed under `UNKNOWN` rather than",
            "  dropped.",
        ),
    ]
    return notebook(cells)


# ===========================================================================
# 06 - Submission charts
# ===========================================================================
def build_06() -> dict:
    cells = [
        md(
            "# 06 - Submission Charts",
            "",
            "The headline tables for the submission, assembled from the live",
            "event log and run report: identity, regime time, drawdown summary,",
            "and per-asset attribution. Everything reuses `guardrail_lab` and",
            "renders as text/Markdown so it runs with the base stack.",
            "",
            "**Offline-safe:** every section degrades to a hint when data is",
            "absent.",
        ),
        bootstrap_cell(),
        code(TABLE_HELPER),
        md("## Load everything"),
        code(
            '''
from guardrail_lab.db import load_events, event_counts
from guardrail_lab.loaders import load_run_report
from guardrail_lab.metrics import nav_series, max_drawdown, trade_count
from guardrail_lab.regime_analysis import time_in_regime
from guardrail_lab.drawdown import analyze_drawdown_from_events
from guardrail_lab.attribution import trade_attribution

events = load_events(str(DB_PATH)) if DB_PATH else []
report = load_run_report(str(RUN_REPORT_PATH)) if RUN_REPORT_PATH else None

if not events and report is None:
    print(NO_DATA_HINT)
else:
    print(f"Events: {len(events)}  |  Run report: "
          f"{'loaded' if report else '(none)'}")
'''
        ),
        md("## Headline: agent identity & status"),
        code(
            '''
if report:
    ident = [
        {"field": "run_id", "value": report.get("run_id", "n/a")},
        {"field": "mode", "value": report.get("mode", "n/a")},
        {"field": "wallet_address", "value": report.get("wallet_address", "n/a")},
        {"field": "policy_hash", "value": report.get("policy_hash", "n/a")},
        {"field": "kill_switch", "value": report.get("kill_switch", "n/a")},
        {"field": "starting_nav_usd", "value": report.get("starting_nav_usd", "n/a")},
        {"field": "nav_usd", "value": report.get("nav_usd", "n/a")},
        {"field": "total_drawdown_pct", "value": report.get("total_drawdown_pct", "n/a")},
    ]
    render_table(ident, [("field", "FIELD"), ("value", "VALUE")],
                 title="Agent identity & status")
else:
    print("No run report available;", NO_DATA_HINT)
'''
        ),
        md("## Headline: performance summary"),
        code(
            '''
if events:
    curve = nav_series(events)
    navs = [nav for _, nav in curve]
    perf = []
    if navs:
        ret = (navs[-1] - navs[0]) / navs[0] * 100.0 if navs[0] else 0.0
        perf.append({"metric": "starting_nav_usd", "value": f"{navs[0]:,.2f}"})
        perf.append({"metric": "latest_nav_usd", "value": f"{navs[-1]:,.2f}"})
        perf.append({"metric": "total_return_pct", "value": f"{ret:+.3f}%"})
        perf.append({"metric": "max_drawdown_pct", "value": f"{max_drawdown(navs) * 100.0:.3f}%"})
    perf.append({"metric": "confirmed_trades", "value": str(trade_count(events))})
    render_table(perf, [("metric", "METRIC"), ("value", "VALUE")],
                 title="Performance summary")
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Headline: time in regime"),
        code(
            '''
if events:
    rows = time_in_regime(events)
    if rows:
        render_table(
            [
                {
                    "regime": t.regime,
                    "classifications": t.classifications,
                    "share": f"{t.fraction:.1%}",
                    "seconds": f"{t.seconds:.1f}",
                }
                for t in rows
            ],
            [
                ("regime", "REGIME"),
                ("classifications", "COUNT"),
                ("share", "SHARE"),
                ("seconds", "SECONDS"),
            ],
            title="Time in regime",
        )
    else:
        print("No regime classifications in the log yet.")
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Headline: drawdown summary"),
        code(
            '''
if events:
    dd = analyze_drawdown_from_events(events, top_n=3)
    if dd.points:
        print(f"Max drawdown: {dd.max_drawdown_pct:.4f}%  "
              f"(peak {dd.peak_timestamp or 'n/a'} -> trough {dd.trough_timestamp or 'n/a'})")
        if dd.episodes:
            render_table(
                [
                    {
                        "depth": f"{ep.depth_pct:.4f}%",
                        "trough": ep.trough_timestamp,
                        "recovered": ep.recovered,
                    }
                    for ep in dd.episodes
                ],
                [("depth", "DEPTH"), ("trough", "TROUGH_TS"), ("recovered", "RECOVERED")],
                title="Top drawdown episodes",
            )
        else:
            print("No drawdown episodes (no decline below a prior peak).")
    else:
        print("No NAV history for drawdown analysis yet.")
else:
    print(NO_DATA_HINT)
'''
        ),
        md("## Headline: trade attribution summary"),
        code(
            '''
if events:
    attribution = trade_attribution(events)
    if attribution:
        total_usd = sum(a["total_amount_usd"] for a in attribution)
        render_table(
            [
                {
                    "symbol": a["symbol"],
                    "trades": a["count"],
                    "notional_usd": f"{a['total_amount_usd']:,.2f}",
                    "share": f"{(a['total_amount_usd'] / total_usd * 100.0):.1f}%" if total_usd else "n/a",
                }
                for a in attribution
            ],
            [
                ("symbol", "SYMBOL"),
                ("trades", "TRADES"),
                ("notional_usd", "NOTIONAL_USD"),
                ("share", "SHARE"),
            ],
            title="Trade attribution",
        )
    else:
        print("No confirmed trades in the log yet.")
else:
    print(NO_DATA_HINT)
'''
        ),
        md(
            "## Notes",
            "",
            "These tables are the offline-safe, dependency-free version of the",
            "submission headline charts. For richer HTML/PNG output the",
            "`guardrail_lab.submission` and `guardrail_lab.charts` modules build",
            "on the same underlying functions when the optional plotting stack",
            "is installed.",
        ),
    ]
    return notebook(cells)


def main() -> None:
    notebooks = {
        "01_universe_filtering.ipynb": build_01(),
        "02_signal_research.ipynb": build_02(),
        "03_backtest_review.ipynb": build_03(),
        "04_live_pnl_analysis.ipynb": build_04(),
        "05_trade_attribution.ipynb": build_05(),
        "06_submission_charts.ipynb": build_06(),
    }
    for name, doc in notebooks.items():
        path = NB_DIR / name
        path.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n",
                        encoding="utf-8")
        print("wrote", path)


if __name__ == "__main__":
    main()
