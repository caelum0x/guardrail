"""Load and compare experiment run files produced by the Rust CLI.

The CLI writes one JSON file per experiment to ``data/experiments/<tag>.json``
with the shape::

    {
      "tag": "...",
      "created_ms": "...",
      "steps": 40,
      "fear_greed": 60,
      "preset": "...",
      "metrics": {
        "total_return_pct": ...,
        "max_drawdown_pct": ...,
        "trade_count": ...,
        "win_rate_pct": ...,
        "profit_factor": ...,
        "volatility_pct": ...,
        "calmar_ratio": ...
      },
      "benchmark_return_pct": ...,
      "excess_return_pct": ...,
      "final_nav_usd": ...
    }

Numeric fields may be serialized either as JSON numbers or as strings, so all
parsing is done defensively.

Standard-library only (json, pathlib).
"""

import json
from pathlib import Path


def _to_float(value: object) -> float | None:
    """Coerce a JSON number-or-string into a float, or ``None`` if not numeric."""
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value.strip())
        except ValueError:
            return None
    return None


def _to_int(value: object) -> int | None:
    """Coerce a JSON number-or-string into an int, or ``None`` if not numeric."""
    as_float = _to_float(value)
    if as_float is None:
        return None
    return int(as_float)


def load_experiments(dir: str = "data/experiments") -> list[dict]:
    """Load every ``*.json`` experiment file in ``dir``.

    Files that are missing, unreadable, or not a JSON object are skipped.
    Results are sorted ascending by ``created_ms`` (treated numerically when
    possible, otherwise by string order, with unparseable values last).
    """
    base = Path(dir)
    if not base.is_dir():
        return []

    experiments: list[dict] = []
    for json_path in sorted(base.glob("*.json")):
        try:
            with json_path.open("r", encoding="utf-8") as handle:
                data = json.load(handle)
        except (json.JSONDecodeError, OSError, UnicodeDecodeError):
            continue
        if not isinstance(data, dict):
            continue
        experiments.append(data)

    def _sort_key(experiment: dict) -> tuple[int, float, str]:
        raw = experiment.get("created_ms")
        as_float = _to_float(raw)
        if as_float is not None:
            return (0, as_float, "")
        return (1, 0.0, str(raw))

    experiments.sort(key=_sort_key)
    return experiments


def compare_table(experiments: list[dict]) -> list[dict]:
    """Build comparison rows (tag + key metrics) from loaded experiments.

    Numeric fields are parsed from strings where necessary. Each row is a flat
    dict with the most decision-relevant fields for a side-by-side view.
    """
    rows: list[dict] = []
    for experiment in experiments:
        metrics = experiment.get("metrics")
        if not isinstance(metrics, dict):
            metrics = {}

        rows.append(
            {
                "tag": str(experiment.get("tag", "")),
                "preset": str(experiment.get("preset", "")),
                "steps": _to_int(experiment.get("steps")),
                "fear_greed": _to_int(experiment.get("fear_greed")),
                "created_ms": _to_int(experiment.get("created_ms")),
                "total_return_pct": _to_float(metrics.get("total_return_pct")),
                "max_drawdown_pct": _to_float(metrics.get("max_drawdown_pct")),
                "trade_count": _to_int(metrics.get("trade_count")),
                "win_rate_pct": _to_float(metrics.get("win_rate_pct")),
                "profit_factor": _to_float(metrics.get("profit_factor")),
                "volatility_pct": _to_float(metrics.get("volatility_pct")),
                "calmar_ratio": _to_float(metrics.get("calmar_ratio")),
                "benchmark_return_pct": _to_float(
                    experiment.get("benchmark_return_pct")
                ),
                "excess_return_pct": _to_float(experiment.get("excess_return_pct")),
                "final_nav_usd": _to_float(experiment.get("final_nav_usd")),
            }
        )
    return rows
