"""Load and validate the Track-2 strategy-skill examples.

The skill under ``skills/cmc-regime-routed-alpha`` ships small JSON example
files describing a target portfolio for a given market regime, e.g.::

    { "market_regime": "risk_on", "target_portfolio": [{ "symbol": "CAKE", "weight_pct": 12 }] }

This module loads those examples and runs lightweight, shape-tolerant
validation so the skill can be backtested for obvious authoring mistakes
(missing regime, empty portfolio, weights that do not sum to ~100, missing
entry/exit rules). It is intentionally forgiving: a well-formed example
returns an empty issue list.

Standard-library only (json, pathlib). No YAML is parsed here.
"""

import json
from pathlib import Path

# Regime can appear under either of these keys depending on the example shape.
_REGIME_KEYS = ("market_regime", "regime")
# Portfolio can appear under either of these keys.
_PORTFOLIO_KEYS = ("target_portfolio", "portfolio")
# Acceptable tolerance (in percentage points) for the weight sum vs 100.
_WEIGHT_TOLERANCE_PCT = 1.0


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


def _first_present(example: dict, keys: tuple[str, ...]) -> object:
    """Return the first value present (and not None) for any of ``keys``."""
    for key in keys:
        if key in example and example[key] is not None:
            return example[key]
    return None


def load_skill_examples(
    skill_dir: str = "skills/cmc-regime-routed-alpha/examples",
) -> list[dict]:
    """Load every ``*.json`` example file in ``skill_dir``.

    Each loaded example is annotated with a ``_source`` key holding the file
    name (so reports can identify it). Files that are missing, unreadable, or
    not a JSON object are skipped. Results are sorted by file name for stable
    output.
    """
    base = Path(skill_dir)
    if not base.is_dir():
        return []

    examples: list[dict] = []
    for json_path in sorted(base.glob("*.json")):
        try:
            with json_path.open("r", encoding="utf-8") as handle:
                data = json.load(handle)
        except (json.JSONDecodeError, OSError, UnicodeDecodeError):
            continue
        if not isinstance(data, dict):
            continue
        annotated = dict(data)
        annotated["_source"] = json_path.name
        examples.append(annotated)

    return examples


def _validate_portfolio(portfolio: object) -> list[str]:
    """Validate a target-portfolio list, returning a list of issues."""
    issues: list[str] = []

    if portfolio is None:
        issues.append("missing target_portfolio")
        return issues
    if not isinstance(portfolio, list):
        issues.append("target_portfolio is not a list")
        return issues
    if not portfolio:
        issues.append("empty target_portfolio")
        return issues

    weight_sum = 0.0
    have_any_weight = False
    for index, position in enumerate(portfolio):
        if not isinstance(position, dict):
            issues.append(f"position {index} is not an object")
            continue
        symbol = position.get("symbol")
        if not isinstance(symbol, str) or not symbol.strip():
            issues.append(f"position {index} missing symbol")
        weight = _to_float(position.get("weight_pct"))
        if weight is None:
            issues.append(f"position {index} missing weight_pct")
        else:
            have_any_weight = True
            weight_sum += weight

    # The strategy intentionally holds a stable reserve, so risk-position
    # weights legitimately sum to <= 100 (the remainder is the reserve). Only
    # an over-allocated book (sum > 100) is an error.
    if have_any_weight and weight_sum > 100.0 + _WEIGHT_TOLERANCE_PCT:
        issues.append(f"weights sum to {weight_sum:g}, over-allocated (> 100)")

    return issues


def validate_example(example: dict) -> list[str]:
    """Validate a single skill example, returning a list of human-readable issues.

    Checks performed (all tolerant of shape differences):

    * a market regime is present and non-empty;
    * a target portfolio is present, a non-empty list, with each position
      carrying a symbol and a numeric ``weight_pct``;
    * portfolio risk weights do not over-allocate (sum <= 100; the remainder
      is the held stable reserve);
    * if the example declares trade rules, both ``entry`` and ``exit`` are
      present (only enforced when a rules block is present).

    Returns ``[]`` when the example looks well-formed.
    """
    if not isinstance(example, dict):
        return ["example is not an object"]

    issues: list[str] = []

    # Examples may carry the decision at the top level or nested under
    # computed/decision/inputs; search all of those scopes.
    scopes: list[dict] = [example]
    for nested in ("computed", "decision", "inputs"):
        value = example.get(nested)
        if isinstance(value, dict):
            scopes.append(value)

    def _find(keys: tuple[str, ...]):
        for scope in scopes:
            found = _first_present(scope, keys)
            if found is not None:
                return found
        return None

    regime = _find(_REGIME_KEYS)
    if regime is None or (isinstance(regime, str) and not regime.strip()):
        issues.append("missing regime")

    portfolio = _find(_PORTFOLIO_KEYS)
    issues.extend(_validate_portfolio(portfolio))

    # Entry/exit rules are optional, but if a rules block is declared we expect
    # both sides to be specified. Accept either nested ("rules": {...}) or flat
    # ("entry"/"exit") shapes.
    rules = example.get("rules")
    has_rules_block = isinstance(rules, dict)
    if has_rules_block:
        if not rules.get("entry"):
            issues.append("entry missing")
        if not rules.get("exit"):
            issues.append("exit missing")
    elif "entry" in example or "exit" in example:
        if not example.get("entry"):
            issues.append("entry missing")
        if not example.get("exit"):
            issues.append("exit missing")

    return issues
