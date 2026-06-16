"""Monte Carlo simulation of strategy equity outcomes. Pure stdlib.

Two engines:
- bootstrap: resample (with replacement) from a historical return series — makes
  no distributional assumption, preserves the empirical return shape.
- gbm: geometric Brownian motion from a drift (mu) and volatility (sigma).

Both project `horizon` steps forward over `paths` simulations and summarize the
terminal-value distribution plus path-level risk (max drawdown, ruin).
"""

from __future__ import annotations

import math
import random
import statistics
from dataclasses import dataclass, field
from typing import Sequence


@dataclass
class SimResult:
    paths: int
    horizon: int
    start: float
    terminal: list[float] = field(default_factory=list)
    max_drawdowns: list[float] = field(default_factory=list)
    ruin_threshold: float = 0.5

    def percentile(self, values: Sequence[float], q: float) -> float:
        if not values:
            return 0.0
        ordered = sorted(values)
        idx = min(max(int(q * (len(ordered) - 1)), 0), len(ordered) - 1)
        return ordered[idx]

    def summary(self) -> dict:
        term = self.terminal
        rets = [t / self.start - 1.0 for t in term] if self.start else []
        ruin = sum(1 for t in term if t <= self.start * self.ruin_threshold) / len(term) if term else 0.0
        return {
            "paths": self.paths,
            "horizon": self.horizon,
            "start": self.start,
            "terminal": {
                "p5": self.percentile(term, 0.05),
                "p50": self.percentile(term, 0.50),
                "p95": self.percentile(term, 0.95),
                "mean": statistics.fmean(term) if term else 0.0,
            },
            "return": {
                "p5": self.percentile(rets, 0.05),
                "p50": self.percentile(rets, 0.50),
                "p95": self.percentile(rets, 0.95),
                "mean": statistics.fmean(rets) if rets else 0.0,
            },
            "max_drawdown_p95": self.percentile(self.max_drawdowns, 0.95),
            "prob_loss": sum(1 for r in rets if r < 0) / len(rets) if rets else 0.0,
            "prob_ruin": ruin,
            "ruin_threshold": self.ruin_threshold,
        }


def _max_drawdown(path: Sequence[float]) -> float:
    peak, mdd = float("-inf"), 0.0
    for v in path:
        peak = max(peak, v)
        if peak > 0:
            mdd = max(mdd, (peak - v) / peak)
    return mdd


def returns_from_equity(equity: Sequence[float]) -> list[float]:
    return [cur / prev - 1.0 for prev, cur in zip(equity, equity[1:]) if prev]


def bootstrap(
    history: Sequence[float],
    paths: int,
    horizon: int,
    start: float = 10_000.0,
    seed: int = 1,
    ruin_threshold: float = 0.5,
) -> SimResult:
    """Bootstrap-resample returns from `history` to project forward."""
    if len(history) < 1:
        raise ValueError("need at least one historical return to bootstrap")
    rng = random.Random(seed)
    res = SimResult(paths=paths, horizon=horizon, start=start, ruin_threshold=ruin_threshold)
    for _ in range(paths):
        equity = start
        path = [equity]
        for _ in range(horizon):
            r = rng.choice(history)
            equity = max(0.0, equity * (1.0 + r))
            path.append(equity)
        res.terminal.append(equity)
        res.max_drawdowns.append(_max_drawdown(path))
    return res


def gbm(
    mu: float,
    sigma: float,
    paths: int,
    horizon: int,
    start: float = 10_000.0,
    seed: int = 1,
    ruin_threshold: float = 0.5,
) -> SimResult:
    """Geometric Brownian motion: per-step return ~ Normal(mu, sigma)."""
    rng = random.Random(seed)
    res = SimResult(paths=paths, horizon=horizon, start=start, ruin_threshold=ruin_threshold)
    for _ in range(paths):
        equity = start
        path = [equity]
        for _ in range(horizon):
            shock = rng.gauss(mu, sigma)
            equity = max(0.0, equity * (1.0 + shock))
            path.append(equity)
        res.terminal.append(equity)
        res.max_drawdowns.append(_max_drawdown(path))
    return res


def estimate_mu_sigma(history: Sequence[float]) -> tuple[float, float]:
    """Sample mean and stdev of a return series (for seeding GBM)."""
    if len(history) < 2:
        return (0.0, 0.0)
    return (statistics.fmean(history), statistics.pstdev(history))
