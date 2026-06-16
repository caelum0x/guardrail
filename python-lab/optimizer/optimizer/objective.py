"""Objective functions over an equity curve. Pure stdlib."""

from __future__ import annotations

import math
from typing import Sequence

PERIODS_PER_YEAR = 365.0


def _returns(equity: Sequence[float]) -> list[float]:
    return [cur / prev - 1.0 for prev, cur in zip(equity, equity[1:]) if prev]


def max_drawdown(equity: Sequence[float]) -> float:
    peak, mdd = float("-inf"), 0.0
    for v in equity:
        peak = max(peak, v)
        if peak > 0:
            mdd = max(mdd, (peak - v) / peak)
    return mdd


def annualized_return(equity: Sequence[float]) -> float:
    n = len(equity) - 1
    if n <= 0 or equity[0] <= 0 or equity[-1] <= 0:
        return 0.0
    return (equity[-1] / equity[0]) ** (PERIODS_PER_YEAR / n) - 1.0


def sharpe(equity: Sequence[float]) -> float:
    rets = _returns(equity)
    if len(rets) < 2:
        return 0.0
    mean = sum(rets) / len(rets)
    var = sum((r - mean) ** 2 for r in rets) / (len(rets) - 1)
    sd = math.sqrt(var)
    return 0.0 if sd == 0 else mean / sd * math.sqrt(PERIODS_PER_YEAR)


def calmar(equity: Sequence[float]) -> float:
    mdd = max_drawdown(equity)
    return 0.0 if mdd == 0 else annualized_return(equity) / mdd


METRICS = {"calmar": calmar, "sharpe": sharpe}


def score_equity(equity: Sequence[float], metric: str) -> float:
    fn = METRICS.get(metric)
    if fn is None:
        raise ValueError(f"unknown metric '{metric}' (choose from {sorted(METRICS)})")
    return fn(equity)
