"""Grid and random search over a strategy parameter space. Pure stdlib."""

from __future__ import annotations

import itertools
import random
from typing import Callable, Mapping, Sequence

# A scorer maps a parameter set to a single number to maximize.
Scorer = Callable[[Mapping[str, float]], float]


def grid_search(space: Mapping[str, Sequence[float]], score: Scorer) -> list[dict]:
    """Score the full Cartesian product of `space`, ranked best (highest) first.

    Returns a list of {"params": {...}, "score": float}.
    """
    keys = list(space.keys())
    results: list[dict] = []
    for combo in itertools.product(*(space[k] for k in keys)):
        params = dict(zip(keys, combo))
        results.append({"params": params, "score": score(params)})
    results.sort(key=lambda r: r["score"], reverse=True)
    return results


def random_search(
    space: Mapping[str, Sequence[float]],
    score: Scorer,
    n: int,
    seed: int = 1,
) -> list[dict]:
    """Sample `n` random points from `space` (with a fixed seed), ranked best first."""
    rng = random.Random(seed)
    keys = list(space.keys())
    seen: set[tuple] = set()
    results: list[dict] = []
    for _ in range(n):
        combo = tuple(rng.choice(list(space[k])) for k in keys)
        if combo in seen:
            continue
        seen.add(combo)
        params = dict(zip(keys, combo))
        results.append({"params": params, "score": score(params)})
    results.sort(key=lambda r: r["score"], reverse=True)
    return results
