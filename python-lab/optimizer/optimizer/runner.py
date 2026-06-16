"""Scorers: an offline synthetic scorer and a live backtest-API scorer.

Both return a single number (higher is better) for a parameter set. Stdlib only
(urllib for the API path) so the optimizer runs with no dependencies.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Mapping

from . import model, objective


def offline_scorer(metric: str):
    """A scorer over the deterministic synthetic equity model."""

    def score(params: Mapping[str, float]) -> float:
        equity = model.synthetic_equity(params)
        return objective.score_equity(equity, metric)

    return score


def api_backtest(skill: str, preset: str, metric: str, base_url: str, timeout: float = 15.0) -> float:
    """Score one named preset by calling the live backtest API.

    GET {base}/skills/{skill}/backtest?preset={preset} -> metrics; we read the
    chosen metric (calmar/sharpe), falling back to total_return_pct / max_drawdown
    if the server reports raw fields.
    """
    url = f"{base_url.rstrip('/')}/skills/{skill}/backtest?preset={preset}"
    try:
        with urllib.request.urlopen(url, timeout=timeout) as resp:  # noqa: S310 - trusted local API
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, json.JSONDecodeError, ValueError) as err:
        raise RuntimeError(f"backtest API call failed: {err}") from err

    metrics = data.get("metrics", data)
    if metric in metrics:
        return float(metrics[metric])
    # Derive a Calmar-like score from raw fields if present.
    ret = _num(metrics, "total_return_pct")
    dd = _num(metrics, "max_drawdown_pct")
    if metric == "calmar" and dd and dd != 0:
        return ret / abs(dd)
    return ret


def _num(d: dict, key: str) -> float:
    v = d.get(key)
    try:
        return float(v)
    except (TypeError, ValueError):
        return 0.0
