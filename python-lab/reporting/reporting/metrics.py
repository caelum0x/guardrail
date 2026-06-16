"""Inline summary risk metrics computed from a NAV series.

Everything here is implemented from first principles using ``decimal.Decimal``
and the standard library only — no numpy, no pandas.

Metrics computed:

* total return       — (last NAV / first NAV) - 1
* max drawdown       — largest peak-to-trough decline over the series
* simple sharpe      — mean(period return) / stdev(period return), where the
                       period returns are the simple returns between successive
                       NAV observations. A risk-free rate of zero is assumed and
                       the ratio is reported un-annualised ("simple sharpe").
* volatility         — sample standard deviation of the period returns

All ratios are expressed as plain Decimals; percentage helpers multiply by 100.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal, getcontext
from typing import Optional, Sequence

from .data import EventLog, NavPoint, RunReport

# Generous precision for chained decimal arithmetic.
getcontext().prec = 50

_ZERO = Decimal(0)
_ONE = Decimal(1)
_HUNDRED = Decimal(100)


@dataclass(frozen=True)
class ReportMetrics:
    """Computed summary metrics. Decimal ratios; *_pct fields are percentages."""

    nav_points: int
    first_nav: Optional[Decimal]
    last_nav: Optional[Decimal]
    peak_nav: Optional[Decimal]
    trough_nav: Optional[Decimal]

    total_return: Optional[Decimal]        # ratio, e.g. -0.00126
    total_return_pct: Optional[Decimal]    # percent, e.g. -0.126
    max_drawdown: Optional[Decimal]        # ratio, non-negative
    max_drawdown_pct: Optional[Decimal]    # percent, non-negative
    volatility: Optional[Decimal]          # stdev of period returns (ratio)
    volatility_pct: Optional[Decimal]
    sharpe: Optional[Decimal]              # simple, un-annualised
    mean_return: Optional[Decimal]         # mean period return (ratio)

    total_events: int
    confirmed_trades: int


def _period_returns(navs: Sequence[Decimal]) -> list[Decimal]:
    """Simple returns between successive NAV observations.

    A return is only defined when the prior NAV is non-zero; zero-NAV pivots are
    skipped rather than producing a division error.
    """
    returns: list[Decimal] = []
    for prev, cur in zip(navs, navs[1:]):
        if prev == _ZERO:
            continue
        returns.append((cur / prev) - _ONE)
    return returns


def _mean(values: Sequence[Decimal]) -> Optional[Decimal]:
    if not values:
        return None
    return sum(values, _ZERO) / Decimal(len(values))


def _stdev(values: Sequence[Decimal], mean: Decimal) -> Optional[Decimal]:
    """Sample standard deviation (n-1 denominator). Needs >= 2 points."""
    n = len(values)
    if n < 2:
        return None
    variance = sum(((v - mean) ** 2 for v in values), _ZERO) / Decimal(n - 1)
    if variance <= _ZERO:
        return _ZERO
    return variance.sqrt()


def _max_drawdown(navs: Sequence[Decimal]) -> Optional[Decimal]:
    """Largest peak-to-trough decline as a non-negative ratio.

    Walks the series tracking the running peak; for each point the drawdown is
    (peak - nav) / peak. Returns the worst such value.
    """
    if not navs:
        return None
    peak = navs[0]
    worst = _ZERO
    for nav in navs:
        if nav > peak:
            peak = nav
        if peak > _ZERO:
            dd = (peak - nav) / peak
            if dd > worst:
                worst = dd
    return worst


def compute_metrics(
    event_log: EventLog,
    run_report: Optional[RunReport] = None,
) -> ReportMetrics:
    """Compute summary risk metrics from the event log's NAV series.

    The ``run_report`` is accepted for context (e.g. seeding a starting NAV when
    the event log itself has no reconciled NAV points) but is never mutated.
    """
    points: Sequence[NavPoint] = event_log.nav_series
    navs: list[Decimal] = [p.nav for p in points]

    # If the event log has no NAV points but the run report carries a starting
    # and ending NAV, build a minimal two-point series so we can still report.
    if not navs and run_report is not None:
        start = run_report.starting_nav_usd
        end = run_report.nav_usd
        if start is not None and end is not None:
            navs = [start, end]

    first = navs[0] if navs else None
    last = navs[-1] if navs else None
    peak = max(navs) if navs else None
    trough = min(navs) if navs else None

    total_return: Optional[Decimal] = None
    if first is not None and last is not None and first != _ZERO:
        total_return = (last / first) - _ONE

    returns = _period_returns(navs)
    mean_ret = _mean(returns)
    vol = _stdev(returns, mean_ret) if mean_ret is not None else None

    sharpe: Optional[Decimal] = None
    if mean_ret is not None and vol is not None and vol > _ZERO:
        sharpe = mean_ret / vol

    max_dd = _max_drawdown(navs)

    def pct(x: Optional[Decimal]) -> Optional[Decimal]:
        return None if x is None else x * _HUNDRED

    return ReportMetrics(
        nav_points=len(navs),
        first_nav=first,
        last_nav=last,
        peak_nav=peak,
        trough_nav=trough,
        total_return=total_return,
        total_return_pct=pct(total_return),
        max_drawdown=max_dd,
        max_drawdown_pct=pct(max_dd),
        volatility=vol,
        volatility_pct=pct(vol),
        sharpe=sharpe,
        mean_return=mean_ret,
        total_events=event_log.total_events,
        confirmed_trades=len(event_log.confirmed_trades),
    )
