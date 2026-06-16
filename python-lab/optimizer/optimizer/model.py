"""Deterministic synthetic equity model for offline optimization.

Maps a strategy parameter set to a reproducible equity curve whose quality
(drift vs. volatility) peaks near a hidden "ideal" parameterization. This gives
the optimizer a real, non-trivial surface to search without needing the live
backtest API — same params always yield the same curve.
"""

from __future__ import annotations

import random
from typing import Mapping

# The hidden optimum the search should rediscover.
IDEAL = {
    "min_score_to_enter": 0.6,
    "min_score_to_hold": 0.45,
    "max_positions": 5.0,
    "rebalance_threshold_pct": 3.0,
    "target_stable_reserve_pct": 15.0,
}

# Plausible spans used to normalize each parameter's distance to the ideal.
SPAN = {
    "min_score_to_enter": 0.5,
    "min_score_to_hold": 0.5,
    "max_positions": 8.0,
    "rebalance_threshold_pct": 8.0,
    "target_stable_reserve_pct": 30.0,
}


def _quality(params: Mapping[str, float]) -> float:
    """0..1 quality: 1.0 at the ideal, decaying with normalized distance."""
    dims = 0
    sq = 0.0
    for key, ideal in IDEAL.items():
        if key in params:
            span = SPAN[key] or 1.0
            d = (float(params[key]) - ideal) / span
            sq += d * d
            dims += 1
    if dims == 0:
        return 0.0
    rms = (sq / dims) ** 0.5
    return max(0.0, 1.0 - rms)


def _seed(params: Mapping[str, float]) -> int:
    # Deterministic seed from a stable repr of the params.
    items = ",".join(f"{k}={float(params[k]):.6f}" for k in sorted(params))
    return abs(hash(items)) % (2**32)


def synthetic_equity(params: Mapping[str, float], steps: int = 180, start: float = 10_000.0) -> list[float]:
    """A reproducible equity curve for `params`.

    Higher quality -> higher positive drift and lower volatility, so better
    parameters produce curves with a higher Calmar/Sharpe.
    """
    q = _quality(params)
    rng = random.Random(_seed(params))
    drift = (q - 0.5) * 0.004  # per-step expected return, ~[-0.2%, +0.2%]
    vol = 0.02 * (1.2 - q)  # better params are less volatile
    equity = [start]
    for _ in range(steps):
        shock = rng.gauss(0.0, 1.0) * vol
        ret = drift + shock
        equity.append(max(1.0, equity[-1] * (1.0 + ret)))
    return equity
