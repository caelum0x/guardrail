"""Monte Carlo bootstrap risk analytics over a NAV curve.

Standard-library only (``random``, ``math``, ``dataclasses``). No numpy.

This module takes a NAV curve — a list of ``(timestamp, nav)`` points, typically
produced by :func:`guardrail_lab.metrics.nav_series` from the event log — derives
the per-step simple returns from the *real* NAV path, then runs an IID
("stationary") bootstrap: it resamples the observed returns with replacement to
build many synthetic forward paths of the same length as the history. From the
ensemble of paths it estimates:

* the distribution of **terminal returns** (cumulative return at the end of each
  path) and its percentiles (p5/p25/p50/p75/p95);
* **percentile NAV cones** — at each forward step, the p5/p25/p50/p75/p95 NAV
  across all paths, anchored at the last observed NAV;
* **VaR / CVaR** at the 95% and 99% confidence levels, both on the terminal
  return *and* on the worst drawdown experienced within each path;
* the **probability of breaching** a drawdown threshold (e.g. the policy
  kill-switch level) at any point along a path.

Design contract:

* Pure and deterministic given the ``seed`` parameter — the same inputs and seed
  always produce identical results (uses :class:`random.Random`, not the global
  RNG, so it never disturbs or is disturbed by other code).
* Frozen, type-annotated dataclasses; never mutates its inputs.
* Never raises on empty or too-short input: it returns a clearly-empty
  :class:`MonteCarloReport` with a human-readable ``reason`` instead.
"""

from __future__ import annotations

import math
import random
from dataclasses import dataclass, field

NavPoint = tuple[str, float]

# Confidence levels reported for VaR / CVaR.
VAR_LEVELS: tuple[float, ...] = (0.95, 0.99)
# Percentiles reported for the terminal-return distribution and NAV cones.
CONE_PERCENTILES: tuple[float, ...] = (5.0, 25.0, 50.0, 75.0, 95.0)

DEFAULT_N_PATHS = 1000
DEFAULT_SEED = 1337
# Default kill-switch-style drawdown breach threshold, as a positive fraction
# (e.g. 0.20 == a 20% peak-to-trough decline).
DEFAULT_DD_THRESHOLD = 0.20


@dataclass(frozen=True)
class TailRisk:
    """Value-at-Risk and Conditional VaR for one confidence level.

    VaR/CVaR are reported on a *loss* convention so that larger numbers always
    mean "worse". For a return series, the loss is ``-return``; for a drawdown
    series, the loss is the drawdown depth itself (already a positive number).

    Attributes:
        level: Confidence level (e.g. ``0.95`` for 95%).
        var: Value-at-Risk — the loss at the ``level`` quantile, as a positive
            fraction. A value of ``0.12`` at level ``0.95`` means "5% of paths
            lose more than 12%".
        cvar: Conditional VaR (expected shortfall) — the mean loss among the
            worst ``(1 - level)`` tail of paths, as a positive fraction. Always
            ``>= var`` (or equal when the tail is a single observation).
    """

    level: float
    var: float
    cvar: float


@dataclass(frozen=True)
class PercentilePoint:
    """One step of a percentile NAV cone.

    Attributes:
        step: 1-based forward step index (step 1 is the first simulated period
            after the last observed NAV).
        p5: 5th-percentile NAV across paths at this step.
        p25: 25th-percentile NAV across paths at this step.
        p50: Median NAV across paths at this step.
        p75: 75th-percentile NAV across paths at this step.
        p95: 95th-percentile NAV across paths at this step.
    """

    step: int
    p5: float
    p25: float
    p50: float
    p75: float
    p95: float


@dataclass(frozen=True)
class MonteCarloReport:
    """Result of a Monte Carlo bootstrap over a NAV curve.

    An empty result (``ok is False``) is returned for empty / too-short input;
    in that case all numeric collections are empty and ``reason`` explains why.

    Attributes:
        ok: ``True`` when the simulation ran; ``False`` for empty/short input.
        reason: Human-readable explanation when ``ok is False`` (empty string
            otherwise).
        n_paths: Number of simulated paths actually generated.
        horizon: Number of forward steps per path (equals the number of observed
            per-step returns).
        seed: The RNG seed used (echoed for reproducibility).
        start_nav: The last observed NAV, used as the anchor for every path.
        n_returns: Number of per-step returns derived from the NAV history.
        dd_threshold: Drawdown breach threshold used, as a positive fraction.
        terminal_return_percentiles: Map of percentile (5/25/50/75/95) to the
            terminal cumulative return at that percentile, as a fraction.
        terminal_return_mean: Mean terminal return across paths, as a fraction.
        terminal_nav_percentiles: Map of percentile to terminal NAV at that
            percentile.
        nav_cone: Per-step percentile NAV cone (length ``horizon``).
        var_terminal: VaR/CVaR on terminal *losses* (``-terminal_return``),
            keyed by confidence level.
        var_drawdown: VaR/CVaR on the worst within-path drawdown, keyed by
            confidence level.
        prob_breach: Probability (0..1) that a path breaches ``dd_threshold``
            at any step.
        worst_drawdowns_mean: Mean across paths of each path's worst drawdown,
            as a positive fraction.
    """

    ok: bool = False
    reason: str = ""
    n_paths: int = 0
    horizon: int = 0
    seed: int = DEFAULT_SEED
    start_nav: float = 0.0
    n_returns: int = 0
    dd_threshold: float = DEFAULT_DD_THRESHOLD
    terminal_return_percentiles: dict[float, float] = field(default_factory=dict)
    terminal_return_mean: float = 0.0
    terminal_nav_percentiles: dict[float, float] = field(default_factory=dict)
    nav_cone: list[PercentilePoint] = field(default_factory=list)
    var_terminal: dict[float, TailRisk] = field(default_factory=dict)
    var_drawdown: dict[float, TailRisk] = field(default_factory=dict)
    prob_breach: float = 0.0
    worst_drawdowns_mean: float = 0.0


def nav_to_returns(nav_curve: list[NavPoint]) -> list[float]:
    """Derive per-step simple returns from a NAV curve.

    The simple return for step ``i`` is ``nav[i] / nav[i - 1] - 1``. Steps whose
    prior NAV is not strictly positive are skipped (a non-positive base makes the
    return undefined), so the result may be shorter than ``len(nav_curve) - 1``.
    The input is not mutated.

    Args:
        nav_curve: Ordered ``(timestamp, nav)`` points.

    Returns:
        The list of per-step simple returns as fractions (empty when fewer than
        two usable points exist).
    """
    returns: list[float] = []
    previous: float | None = None
    for _timestamp, nav in nav_curve:
        if previous is not None and previous > 0.0:
            returns.append(nav / previous - 1.0)
        previous = nav
    return returns


def _percentile(sorted_values: list[float], pct: float) -> float:
    """Linear-interpolation percentile of an already-sorted list.

    Uses the same convention as ``numpy.percentile`` default (linear), but on
    pure Python. ``pct`` is in ``[0, 100]``. Returns ``0.0`` for an empty list.
    """
    if not sorted_values:
        return 0.0
    if len(sorted_values) == 1:
        return sorted_values[0]
    rank = (pct / 100.0) * (len(sorted_values) - 1)
    low = math.floor(rank)
    high = math.ceil(rank)
    if low == high:
        return sorted_values[int(rank)]
    frac = rank - low
    return sorted_values[low] * (1.0 - frac) + sorted_values[high] * frac


def _tail_risk(losses: list[float], level: float) -> TailRisk:
    """Compute VaR/CVaR on a list of losses at a confidence level.

    Args:
        losses: Per-path losses (positive == worse). Need not be sorted.
        level: Confidence level in ``(0, 1)`` (e.g. ``0.95``).

    Returns:
        A :class:`TailRisk`. VaR is the ``level`` quantile of the loss
        distribution; CVaR is the mean of losses at or beyond VaR (the worst
        ``1 - level`` tail). Empty input yields zeros.
    """
    if not losses:
        return TailRisk(level=level, var=0.0, cvar=0.0)
    ordered = sorted(losses)
    var = _percentile(ordered, level * 100.0)
    tail = [loss for loss in ordered if loss >= var]
    if not tail:
        tail = [ordered[-1]]
    cvar = sum(tail) / len(tail)
    return TailRisk(level=level, var=var, cvar=cvar)


def _simulate_path(
    returns: list[float],
    start_nav: float,
    horizon: int,
    rng: random.Random,
) -> tuple[float, list[float], float]:
    """Generate one bootstrapped NAV path.

    Resamples ``horizon`` returns from ``returns`` with replacement and compounds
    them onto ``start_nav``.

    Returns:
        A tuple of ``(terminal_return, nav_path, worst_drawdown)`` where
        ``nav_path`` has one NAV per forward step (length ``horizon``) and
        ``worst_drawdown`` is the deepest peak-to-trough decline along the path
        as a positive fraction.
    """
    nav = start_nav
    peak = start_nav
    worst_dd = 0.0
    nav_path: list[float] = []
    for _ in range(horizon):
        step_return = returns[rng.randrange(len(returns))]
        nav *= 1.0 + step_return
        nav_path.append(nav)
        if nav > peak:
            peak = nav
        if peak > 0.0:
            drawdown = (peak - nav) / peak
            if drawdown > worst_dd:
                worst_dd = drawdown
    terminal_return = (nav / start_nav - 1.0) if start_nav > 0.0 else 0.0
    return terminal_return, nav_path, worst_dd


def bootstrap(
    nav_curve: list[NavPoint],
    n_paths: int = DEFAULT_N_PATHS,
    horizon: int | None = None,
    seed: int = DEFAULT_SEED,
    dd_threshold: float = DEFAULT_DD_THRESHOLD,
) -> MonteCarloReport:
    """Run an IID bootstrap risk simulation over a NAV curve.

    Derives per-step returns from ``nav_curve`` (via :func:`nav_to_returns`),
    then builds ``n_paths`` synthetic forward NAV paths by resampling those
    returns with replacement, each of length ``horizon`` (defaulting to the
    number of observed returns). The last observed NAV anchors every path.

    The simulation is fully deterministic given ``seed``: it uses a private
    :class:`random.Random` instance, so it neither reads nor mutates global RNG
    state.

    Never raises on empty or too-short input. If there are fewer than two usable
    NAV points (so no returns can be derived), or if ``n_paths``/``horizon`` are
    non-positive, it returns a :class:`MonteCarloReport` with ``ok is False`` and
    a ``reason`` describing the problem.

    Args:
        nav_curve: Ordered ``(timestamp, nav)`` points (the real NAV history).
        n_paths: Number of bootstrap paths to generate (default ``1000``).
        horizon: Steps per path; defaults to the number of observed returns.
        seed: RNG seed for reproducibility.
        dd_threshold: Drawdown breach level as a positive fraction (e.g. ``0.20``
            for a 20% kill-switch). Values are clamped to ``>= 0``.

    Returns:
        A populated :class:`MonteCarloReport` (``ok is True``) on success, or a
        clearly-empty one (``ok is False``) with a ``reason`` otherwise.
    """
    returns = nav_to_returns(nav_curve)
    n_returns = len(returns)
    start_nav = nav_curve[-1][1] if nav_curve else 0.0
    effective_horizon = n_returns if horizon is None else horizon
    threshold = max(0.0, dd_threshold)

    if not nav_curve:
        return MonteCarloReport(
            ok=False,
            reason="empty NAV curve — no points supplied.",
            seed=seed,
            dd_threshold=threshold,
        )
    if n_returns < 1:
        return MonteCarloReport(
            ok=False,
            reason=(
                "not enough NAV points to derive returns "
                "(need at least two with a positive prior NAV)."
            ),
            seed=seed,
            start_nav=start_nav,
            n_returns=n_returns,
            dd_threshold=threshold,
        )
    if n_paths < 1:
        return MonteCarloReport(
            ok=False,
            reason=f"n_paths must be >= 1 (got {n_paths}).",
            seed=seed,
            start_nav=start_nav,
            n_returns=n_returns,
            dd_threshold=threshold,
        )
    if effective_horizon < 1:
        return MonteCarloReport(
            ok=False,
            reason=f"horizon must be >= 1 (got {effective_horizon}).",
            seed=seed,
            start_nav=start_nav,
            n_returns=n_returns,
            dd_threshold=threshold,
        )

    rng = random.Random(seed)

    terminal_returns: list[float] = []
    worst_drawdowns: list[float] = []
    breaches = 0
    # Per-step NAV samples across all paths, for the cone.
    step_navs: list[list[float]] = [[] for _ in range(effective_horizon)]

    for _ in range(n_paths):
        terminal_return, nav_path, worst_dd = _simulate_path(
            returns, start_nav, effective_horizon, rng
        )
        terminal_returns.append(terminal_return)
        worst_drawdowns.append(worst_dd)
        if worst_dd >= threshold:
            breaches += 1
        for step_index, nav in enumerate(nav_path):
            step_navs[step_index].append(nav)

    sorted_terminal = sorted(terminal_returns)
    terminal_return_percentiles = {
        pct: _percentile(sorted_terminal, pct) for pct in CONE_PERCENTILES
    }
    terminal_nav_percentiles = {
        pct: start_nav * (1.0 + value)
        for pct, value in terminal_return_percentiles.items()
    }
    terminal_return_mean = sum(terminal_returns) / len(terminal_returns)

    nav_cone: list[PercentilePoint] = []
    for step_index, navs in enumerate(step_navs):
        ordered = sorted(navs)
        nav_cone.append(
            PercentilePoint(
                step=step_index + 1,
                p5=_percentile(ordered, 5.0),
                p25=_percentile(ordered, 25.0),
                p50=_percentile(ordered, 50.0),
                p75=_percentile(ordered, 75.0),
                p95=_percentile(ordered, 95.0),
            )
        )

    # Terminal-return tail risk on the loss convention (-return).
    terminal_losses = [-value for value in terminal_returns]
    var_terminal = {
        level: _tail_risk(terminal_losses, level) for level in VAR_LEVELS
    }
    # Drawdown tail risk: drawdowns are already positive losses.
    var_drawdown = {
        level: _tail_risk(worst_drawdowns, level) for level in VAR_LEVELS
    }

    prob_breach = breaches / n_paths
    worst_drawdowns_mean = sum(worst_drawdowns) / len(worst_drawdowns)

    return MonteCarloReport(
        ok=True,
        reason="",
        n_paths=n_paths,
        horizon=effective_horizon,
        seed=seed,
        start_nav=start_nav,
        n_returns=n_returns,
        dd_threshold=threshold,
        terminal_return_percentiles=terminal_return_percentiles,
        terminal_return_mean=terminal_return_mean,
        terminal_nav_percentiles=terminal_nav_percentiles,
        nav_cone=nav_cone,
        var_terminal=var_terminal,
        var_drawdown=var_drawdown,
        prob_breach=prob_breach,
        worst_drawdowns_mean=worst_drawdowns_mean,
    )


def bootstrap_from_events(
    events: list[dict],
    n_paths: int = DEFAULT_N_PATHS,
    horizon: int | None = None,
    seed: int = DEFAULT_SEED,
    dd_threshold: float = DEFAULT_DD_THRESHOLD,
) -> MonteCarloReport:
    """Convenience wrapper: build the NAV curve from events, then bootstrap.

    Reuses :func:`guardrail_lab.metrics.nav_series` to extract the NAV curve from
    ``portfolio_reconciled`` events before delegating to :func:`bootstrap`. Safe
    on empty input (returns an empty report).

    Args:
        events: Parsed event log (e.g. from
            :func:`guardrail_lab.db.load_events`).
        n_paths: Number of bootstrap paths.
        horizon: Steps per path (defaults to the number of observed returns).
        seed: RNG seed for reproducibility.
        dd_threshold: Drawdown breach level as a positive fraction.

    Returns:
        A :class:`MonteCarloReport`.
    """
    from .metrics import nav_series

    return bootstrap(
        nav_series(events),
        n_paths=n_paths,
        horizon=horizon,
        seed=seed,
        dd_threshold=dd_threshold,
    )
