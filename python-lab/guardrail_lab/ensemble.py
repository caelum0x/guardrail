"""Regime-aware meta-allocator that blends the four Track-2 strategy skills.

This module sits *above* the four sibling Track-2 skills
(``cmc-regime-routed-alpha``, ``funding-rate-carry``, ``mean-reversion-chop``,
``trend-breakout-momentum``). For a single classified market regime it:

1. loads the per-regime, per-skill blend weights from ``skills/ensemble.json``,
2. loads each skill's example target portfolio for that regime
   (``skills/<skill>/examples/<regime>_example.json``), and
3. produces the **blended target portfolio** — a weighted average of every
   skill's per-symbol target weight, renormalized so the risk allocation is
   ``<= max_risk_allocation_pct`` with the remainder held in a USDT reserve —
   plus a **per-skill contribution attribution** showing how much risk weight
   each skill contributed.

The blend is *advisory only*: the Rust risk engine remains the sole execution
gate (per-position caps, stable-reserve floor, drawdown kill-switch). This
module never executes anything; it only proposes a target book for the engine
to validate.

Design contract:

* Standard-library only (``json``, ``pathlib``) so it runs without pip
  installs. The example/config files are JSON, so no YAML parser is needed.
* All public functions are pure: they read files once and return frozen
  dataclasses without mutating their inputs.
* Nothing raises on missing/malformed files. A missing config or regime yields
  a clearly-empty :class:`EnsembleResult` carrying a human-readable ``reason``.

Example payload shapes consumed (tolerant of either nesting):

    skills/ensemble.json
        { "regimes": { "risk_on": { "weights": { "<skill>": 0.35, ... },
                                     "rationale": "..." }, ... },
          "reserve_symbol": "USDT", "max_risk_allocation_pct": 100.0 }

    skills/<skill>/examples/<regime>_example.json
        { "decision": { "target_portfolio": [ {"symbol": "CAKE",
                                               "weight_pct": 17.0}, ... ] } }
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

#: Default location of the ensemble blend-weights config.
DEFAULT_CONFIG_PATH = "skills/ensemble.json"
#: Default root holding the four Track-2 skill directories.
DEFAULT_SKILLS_ROOT = "skills"
#: The reserve / quote leg symbol; the remainder of the book is held here.
DEFAULT_RESERVE_SYMBOL = "USDT"
#: Risk allocation can never exceed this (the rest is reserve).
DEFAULT_MAX_RISK_ALLOCATION_PCT = 100.0
#: The four regimes the strategy classifies.
KNOWN_REGIMES = ("risk_on", "risk_off", "chop", "breakout")
#: Keys under which a target portfolio may appear in an example file.
_PORTFOLIO_KEYS = ("target_portfolio", "portfolio")
#: Scopes within an example to search for the target portfolio.
_PORTFOLIO_SCOPES = ("decision", "computed", "inputs")


@dataclass(frozen=True)
class SkillContribution:
    """How a single skill contributed to the blended book for one regime.

    Attributes:
        skill: The skill directory name (e.g. ``"mean-reversion-chop"``).
        blend_weight: The skill's blend weight for this regime, in ``[0, 1]``.
        risk_weight_pct: Total *risk* weight (excluding the reserve symbol) the
            skill's own example portfolio allocated, in percentage points.
        contributed_pct: ``blend_weight * risk_weight_pct`` — the skill's
            pre-renormalization contribution to the blended risk book.
        loaded: ``True`` when the skill's example for this regime loaded; when
            ``False`` the skill contributed nothing (and ``reason`` says why).
        reason: Human-readable note (empty when ``loaded`` is ``True``).
    """

    skill: str
    blend_weight: float
    risk_weight_pct: float
    contributed_pct: float
    loaded: bool
    reason: str = ""


@dataclass(frozen=True)
class BlendedPosition:
    """One line of the blended target portfolio.

    Attributes:
        symbol: Asset symbol (the reserve symbol appears last).
        weight_pct: Final blended target weight in percentage points.
    """

    symbol: str
    weight_pct: float


@dataclass(frozen=True)
class EnsembleResult:
    """The blended target book and attribution for a single regime.

    Attributes:
        regime: The regime this blend was computed for.
        ok: ``True`` when at least one skill example loaded and a non-empty
            blended book was produced.
        reason: Human-readable explanation (empty on success, populated when
            ``ok`` is ``False`` or some skills were skipped).
        rationale: The regime's rationale text copied from the config.
        target_portfolio: The blended target book, risk positions first
            (sorted by descending weight then symbol) with the reserve symbol
            last. Risk weights are renormalized to ``<= max_risk_allocation_pct``
            and the remainder is the reserve line.
        reserve_symbol: The symbol holding the unallocated remainder.
        reserve_pct: The reserve weight in percentage points.
        contributions: Per-skill contribution attribution for this regime.
    """

    regime: str
    ok: bool
    reason: str
    rationale: str = ""
    target_portfolio: list[BlendedPosition] = field(default_factory=list)
    reserve_symbol: str = DEFAULT_RESERVE_SYMBOL
    reserve_pct: float = 0.0
    contributions: list[SkillContribution] = field(default_factory=list)


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


def _load_json(path: Path) -> dict | None:
    """Load a JSON object from ``path``; return ``None`` on any failure."""
    if not path.is_file():
        return None
    try:
        with path.open("r", encoding="utf-8") as handle:
            data = json.load(handle)
    except (json.JSONDecodeError, OSError, UnicodeDecodeError):
        return None
    return data if isinstance(data, dict) else None


def load_config(config_path: str = DEFAULT_CONFIG_PATH) -> dict | None:
    """Load and shallow-validate the ensemble config.

    Returns the parsed config dict, or ``None`` when the file is missing,
    unparseable, or has no ``regimes`` mapping.
    """
    data = _load_json(Path(config_path))
    if data is None:
        return None
    if not isinstance(data.get("regimes"), dict):
        return None
    return data


def _extract_portfolio(example: dict) -> list[dict]:
    """Pull the target-portfolio list from an example, tolerant of nesting."""
    scopes: list[dict] = [example]
    for nested in _PORTFOLIO_SCOPES:
        value = example.get(nested)
        if isinstance(value, dict):
            scopes.append(value)

    for scope in scopes:
        for key in _PORTFOLIO_KEYS:
            value = scope.get(key)
            if isinstance(value, list):
                return [item for item in value if isinstance(item, dict)]
    return []


def load_skill_portfolio(
    skill: str,
    regime: str,
    skills_root: str = DEFAULT_SKILLS_ROOT,
) -> dict[str, float]:
    """Load one skill's example target portfolio for ``regime``.

    Reads ``<skills_root>/<skill>/examples/<regime>_example.json`` and returns
    a ``{symbol: weight_pct}`` mapping. Returns an empty mapping when the file
    is missing/malformed or carries no portfolio (never raises).
    """
    path = Path(skills_root) / skill / "examples" / f"{regime}_example.json"
    example = _load_json(path)
    if example is None:
        return {}

    portfolio: dict[str, float] = {}
    for position in _extract_portfolio(example):
        symbol = position.get("symbol")
        weight = _to_float(position.get("weight_pct"))
        if isinstance(symbol, str) and symbol.strip() and weight is not None:
            portfolio[symbol.strip()] = portfolio.get(symbol.strip(), 0.0) + weight
    return portfolio


def _normalized_blend_weights(raw_weights: dict) -> dict[str, float]:
    """Coerce + renormalize per-skill blend weights to sum to 1.0.

    Non-numeric or negative entries are dropped. When the surviving weights sum
    to a positive value they are renormalized to 1.0 (so the config is robust
    even if hand-edited weights drift); an all-zero/empty set returns ``{}``.
    """
    clean: dict[str, float] = {}
    for skill, value in raw_weights.items():
        weight = _to_float(value)
        if weight is not None and weight > 0:
            clean[skill] = weight
    total = sum(clean.values())
    if total <= 0:
        return {}
    return {skill: weight / total for skill, weight in clean.items()}


def blend_regime(
    regime: str,
    config_path: str = DEFAULT_CONFIG_PATH,
    skills_root: str = DEFAULT_SKILLS_ROOT,
) -> EnsembleResult:
    """Compute the blended target book + attribution for a single regime.

    The blend is a *weighted average of per-skill target weights*: for each
    symbol, the blended risk weight is ``Σ blend_weight[skill] *
    skill_weight_pct[symbol]`` over the risk (non-reserve) positions. Reserve
    weight from each skill is intentionally ignored at blend time and
    recomputed as the renormalized remainder, so the final book always carries
    a single, coherent reserve line of ``max_risk_allocation_pct - Σ risk``.

    Args:
        regime: One of :data:`KNOWN_REGIMES` (other labels are accepted but
            will simply find no per-regime weights and return an empty result).
        config_path: Path to ``skills/ensemble.json``.
        skills_root: Root directory holding the four skill directories.

    Returns:
        An :class:`EnsembleResult`. On any failure (missing config, unknown
        regime, no skill examples) ``ok`` is ``False`` and ``reason`` explains
        why; the function never raises.
    """
    config = load_config(config_path)
    if config is None:
        return EnsembleResult(
            regime=regime,
            ok=False,
            reason=(
                f"ensemble config not found or invalid at '{config_path}' "
                "(expected a JSON object with a 'regimes' mapping)."
            ),
        )

    reserve_symbol = config.get("reserve_symbol") or DEFAULT_RESERVE_SYMBOL
    max_risk = _to_float(config.get("max_risk_allocation_pct"))
    if max_risk is None or max_risk <= 0:
        max_risk = DEFAULT_MAX_RISK_ALLOCATION_PCT

    regime_cfg = config["regimes"].get(regime)
    if not isinstance(regime_cfg, dict):
        available = ", ".join(sorted(config["regimes"])) or "(none)"
        return EnsembleResult(
            regime=regime,
            ok=False,
            reason=(
                f"regime '{regime}' is not configured. "
                f"Available regimes: {available}."
            ),
            reserve_symbol=reserve_symbol,
        )

    rationale = str(regime_cfg.get("rationale") or "")
    blend_weights = _normalized_blend_weights(
        regime_cfg.get("weights") if isinstance(regime_cfg.get("weights"), dict)
        else {}
    )
    if not blend_weights:
        return EnsembleResult(
            regime=regime,
            ok=False,
            reason=f"no valid per-skill blend weights for regime '{regime}'.",
            rationale=rationale,
            reserve_symbol=reserve_symbol,
        )

    # Accumulate the blended risk book and per-skill attribution.
    blended_risk: dict[str, float] = {}
    contributions: list[SkillContribution] = []
    loaded_any = False

    for skill in sorted(blend_weights):
        blend_weight = blend_weights[skill]
        portfolio = load_skill_portfolio(skill, regime, skills_root)

        risk_positions = {
            symbol: weight
            for symbol, weight in portfolio.items()
            if symbol != reserve_symbol
        }
        risk_weight_pct = round(sum(risk_positions.values()), 6)
        loaded = bool(portfolio)
        if loaded:
            loaded_any = True
            for symbol, weight in risk_positions.items():
                blended_risk[symbol] = (
                    blended_risk.get(symbol, 0.0) + blend_weight * weight
                )

        contributions.append(
            SkillContribution(
                skill=skill,
                blend_weight=round(blend_weight, 6),
                risk_weight_pct=risk_weight_pct,
                contributed_pct=round(blend_weight * risk_weight_pct, 6),
                loaded=loaded,
                reason=(
                    ""
                    if loaded
                    else (
                        "no example portfolio found at "
                        f"{skills_root}/{skill}/examples/{regime}_example.json"
                    )
                ),
            )
        )

    if not loaded_any:
        return EnsembleResult(
            regime=regime,
            ok=False,
            reason=(
                f"no skill example portfolios could be loaded for regime "
                f"'{regime}' under '{skills_root}'."
            ),
            rationale=rationale,
            reserve_symbol=reserve_symbol,
            contributions=contributions,
        )

    target_portfolio, reserve_pct = _finalize_book(
        blended_risk, reserve_symbol, max_risk
    )

    skipped = [c.skill for c in contributions if not c.loaded]
    reason = (
        ""
        if not skipped
        else "blended with partial inputs; skipped: " + ", ".join(skipped)
    )

    return EnsembleResult(
        regime=regime,
        ok=True,
        reason=reason,
        rationale=rationale,
        target_portfolio=target_portfolio,
        reserve_symbol=reserve_symbol,
        reserve_pct=reserve_pct,
        contributions=contributions,
    )


def _finalize_book(
    blended_risk: dict[str, float],
    reserve_symbol: str,
    max_risk: float,
) -> tuple[list[BlendedPosition], float]:
    """Renormalize the blended risk book to ``<= max_risk`` and add reserve.

    When the summed risk weight exceeds ``max_risk`` it is scaled down
    proportionally so the book never over-allocates; otherwise it is kept as-is
    and the remainder becomes the reserve. Returns the ordered position list
    (risk positions by descending weight then symbol, reserve last) and the
    reserve percentage.
    """
    total_risk = sum(blended_risk.values())
    if total_risk > max_risk and total_risk > 0:
        scale = max_risk / total_risk
        scaled = {symbol: weight * scale for symbol, weight in blended_risk.items()}
        total_risk = max_risk
    else:
        scaled = dict(blended_risk)

    reserve_pct = round(max(0.0, max_risk - total_risk), 4)

    positions = [
        BlendedPosition(symbol=symbol, weight_pct=round(weight, 4))
        for symbol, weight in scaled.items()
        if round(weight, 4) > 0.0
    ]
    positions.sort(key=lambda position: (-position.weight_pct, position.symbol))

    if reserve_pct > 0.0:
        positions.append(
            BlendedPosition(symbol=reserve_symbol, weight_pct=reserve_pct)
        )

    return positions, reserve_pct


def render_markdown(result: EnsembleResult) -> str:
    """Render an :class:`EnsembleResult` as a readable Markdown report.

    Always returns a non-empty string; on a failed/empty result it renders the
    ``reason`` rather than raising.
    """
    lines: list[str] = []
    lines.append(f"# Ensemble Blend — regime `{result.regime}`")
    lines.append("")

    if result.rationale:
        lines.append(f"> {result.rationale}")
        lines.append("")

    if not result.ok:
        lines.append(f"**No blend produced.** {result.reason}")
        lines.append("")
        return "\n".join(lines)

    if result.reason:
        lines.append(f"_Note: {result.reason}_")
        lines.append("")

    lines.append("## Blended Target Portfolio")
    lines.append("")
    lines.append("| Symbol | Weight % |")
    lines.append("| --- | ---: |")
    for position in result.target_portfolio:
        marker = " (reserve)" if position.symbol == result.reserve_symbol else ""
        lines.append(f"| {position.symbol}{marker} | {position.weight_pct:.2f} |")
    total = sum(p.weight_pct for p in result.target_portfolio)
    lines.append(f"| **total** | **{total:.2f}** |")
    lines.append("")

    lines.append("## Per-Skill Contribution")
    lines.append("")
    lines.append("| Skill | Blend wt | Skill risk % | Contributed % | Loaded |")
    lines.append("| --- | ---: | ---: | ---: | :---: |")
    for contribution in result.contributions:
        loaded = "yes" if contribution.loaded else "no"
        lines.append(
            f"| {contribution.skill} | {contribution.blend_weight:.2f} | "
            f"{contribution.risk_weight_pct:.2f} | "
            f"{contribution.contributed_pct:.2f} | {loaded} |"
        )
    lines.append("")
    lines.append(
        "_Execution note: this blended book is advisory. The Rust risk engine "
        "is the sole execution gate (per-name caps, stable-reserve floor, "
        "drawdown kill-switch) and may clip or reject any position._"
    )
    lines.append("")
    return "\n".join(lines)
