"""Portfolio risk and performance metrics.

Pure standard-library Python (no numpy required), so it runs anywhere. Every
function operates on a list of floats; helpers convert an equity/NAV curve into
a return series first. Formulas are the standard finance definitions and are
covered by the doctests below.
"""

from __future__ import annotations

import math
from typing import Sequence

# Trading periods per year for annualization. Crypto trades 365 days/yr; the
# agent's cycle is sub-daily, but callers can override per their sampling.
DEFAULT_PERIODS_PER_YEAR = 365.0


def returns_from_equity(equity: Sequence[float]) -> list[float]:
    """Simple period-over-period returns from an equity/NAV curve.

    >>> [round(r, 4) for r in returns_from_equity([100, 110, 99])]
    [0.1, -0.1]
    """
    out: list[float] = []
    for prev, cur in zip(equity, equity[1:]):
        if prev == 0:
            out.append(0.0)
        else:
            out.append(cur / prev - 1.0)
    return out


def _mean(xs: Sequence[float]) -> float:
    return sum(xs) / len(xs) if xs else 0.0


def _std(xs: Sequence[float], sample: bool = True) -> float:
    n = len(xs)
    if n < 2:
        return 0.0
    m = _mean(xs)
    denom = n - 1 if sample else n
    return math.sqrt(sum((x - m) ** 2 for x in xs) / denom)


def max_drawdown(equity: Sequence[float]) -> float:
    """Maximum peak-to-trough drawdown as a positive fraction (0.2 == 20%).

    >>> round(max_drawdown([100, 120, 60, 80]), 4)
    0.5
    """
    peak = float("-inf")
    mdd = 0.0
    for v in equity:
        peak = max(peak, v)
        if peak > 0:
            mdd = max(mdd, (peak - v) / peak)
    return mdd


def total_return(equity: Sequence[float]) -> float:
    """Total return over the whole curve.

    >>> total_return([100, 150])
    0.5
    """
    if len(equity) < 2 or equity[0] == 0:
        return 0.0
    return equity[-1] / equity[0] - 1.0


def annualized_return(equity: Sequence[float], periods_per_year: float = DEFAULT_PERIODS_PER_YEAR) -> float:
    """Geometric annualized return from an equity curve."""
    n = len(equity) - 1
    if n <= 0 or equity[0] <= 0 or equity[-1] <= 0:
        return 0.0
    growth = equity[-1] / equity[0]
    return growth ** (periods_per_year / n) - 1.0


def annualized_volatility(returns: Sequence[float], periods_per_year: float = DEFAULT_PERIODS_PER_YEAR) -> float:
    """Annualized standard deviation of returns."""
    return _std(returns) * math.sqrt(periods_per_year)


def sharpe(returns: Sequence[float], rf: float = 0.0, periods_per_year: float = DEFAULT_PERIODS_PER_YEAR) -> float:
    """Annualized Sharpe ratio. `rf` is the per-period risk-free rate."""
    excess = [r - rf for r in returns]
    sd = _std(excess)
    if sd == 0:
        return 0.0
    return _mean(excess) / sd * math.sqrt(periods_per_year)


def sortino(returns: Sequence[float], rf: float = 0.0, periods_per_year: float = DEFAULT_PERIODS_PER_YEAR) -> float:
    """Annualized Sortino ratio — like Sharpe but penalizes only downside vol."""
    excess = [r - rf for r in returns]
    downside = [min(0.0, e) for e in excess]
    dd = math.sqrt(sum(d * d for d in downside) / len(downside)) if downside else 0.0
    if dd == 0:
        return 0.0
    return _mean(excess) / dd * math.sqrt(periods_per_year)


def calmar(equity: Sequence[float], periods_per_year: float = DEFAULT_PERIODS_PER_YEAR) -> float:
    """Calmar ratio: annualized return divided by max drawdown."""
    mdd = max_drawdown(equity)
    if mdd == 0:
        return 0.0
    return annualized_return(equity, periods_per_year) / mdd


def historical_var(returns: Sequence[float], confidence: float = 0.95) -> float:
    """Historical Value-at-Risk as a positive loss fraction at `confidence`.

    The empirical `(1 - confidence)` quantile of the return distribution,
    reported as a positive number (a loss).
    """
    if not returns:
        return 0.0
    ordered = sorted(returns)
    idx = int((1.0 - confidence) * len(ordered))
    idx = min(max(idx, 0), len(ordered) - 1)
    return max(0.0, -ordered[idx])


def parametric_var(returns: Sequence[float], confidence: float = 0.95) -> float:
    """Gaussian (parametric) VaR as a positive loss fraction at `confidence`."""
    if len(returns) < 2:
        return 0.0
    # Inverse-normal CDF (Acklam's rational approximation) for the z-score.
    z = _norm_ppf(1.0 - confidence)
    loss = -(_mean(returns) + z * _std(returns))
    return max(0.0, loss)


def correlation_matrix(series: dict[str, Sequence[float]]) -> dict[str, dict[str, float]]:
    """Pairwise Pearson correlation across named return series (equal lengths)."""
    names = list(series.keys())
    out: dict[str, dict[str, float]] = {a: {} for a in names}
    for a in names:
        for b in names:
            out[a][b] = _pearson(series[a], series[b])
    return out


def _pearson(xs: Sequence[float], ys: Sequence[float]) -> float:
    n = min(len(xs), len(ys))
    if n < 2:
        return 0.0
    xs, ys = xs[:n], ys[:n]
    mx, my = _mean(xs), _mean(ys)
    cov = sum((x - mx) * (y - my) for x, y in zip(xs, ys))
    vx = math.sqrt(sum((x - mx) ** 2 for x in xs))
    vy = math.sqrt(sum((y - my) ** 2 for y in ys))
    if vx == 0 or vy == 0:
        return 0.0
    return cov / (vx * vy)


def _norm_ppf(p: float) -> float:
    """Inverse standard-normal CDF via Acklam's rational approximation."""
    if p <= 0.0:
        return -math.inf
    if p >= 1.0:
        return math.inf
    a = [-3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02,
         1.383577518672690e+02, -3.066479806614716e+01, 2.506628277459239e+00]
    b = [-5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02,
         6.680131188771972e+01, -1.328068155288572e+01]
    c = [-7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00,
         -2.549732539343734e+00, 4.374664141464968e+00, 2.938163982698783e+00]
    d = [7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00,
         3.754408661907416e+00]
    plow, phigh = 0.02425, 1 - 0.02425
    if p < plow:
        q = math.sqrt(-2 * math.log(p))
        return (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / \
               ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1)
    if p > phigh:
        q = math.sqrt(-2 * math.log(1 - p))
        return -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) / \
               ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1)
    q = p - 0.5
    r = q * q
    return (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q / \
           (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1)


def summary(equity: Sequence[float], periods_per_year: float = DEFAULT_PERIODS_PER_YEAR) -> dict[str, float]:
    """Compute the full metric suite for an equity curve."""
    rets = returns_from_equity(equity)
    return {
        "points": len(equity),
        "total_return": total_return(equity),
        "annualized_return": annualized_return(equity, periods_per_year),
        "annualized_volatility": annualized_volatility(rets, periods_per_year),
        "max_drawdown": max_drawdown(equity),
        "sharpe": sharpe(rets, 0.0, periods_per_year),
        "sortino": sortino(rets, 0.0, periods_per_year),
        "calmar": calmar(equity, periods_per_year),
        "var_95_historical": historical_var(rets, 0.95),
        "var_95_parametric": parametric_var(rets, 0.95),
    }


if __name__ == "__main__":
    import doctest

    doctest.testmod(verbose=False)
