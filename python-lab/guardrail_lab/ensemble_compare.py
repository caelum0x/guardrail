"""Compare the blended ensemble book against each single skill, per regime.

This module sits one level *above* :mod:`guardrail_lab.ensemble`. For a single
classified regime it answers a concrete question: *does blending the four
Track-2 skills actually produce a more diversified, better-shaped risk book
than any one skill on its own?*

It reuses, and never recomputes:

* :func:`guardrail_lab.ensemble.blend_regime` for the BLENDED target book, and
* :func:`guardrail_lab.ensemble.load_skill_portfolio` for each single skill's
  own example target book for the same regime.

For each book (the blend, plus every single skill) it derives only
*defensible, data-available* portfolio-shape metrics — nothing is invented or
back-filled with synthetic data:

* **n_positions** — number of distinct risk positions (the reserve symbol,
  e.g. USDT, is excluded; it is a reserve, not a risk bet).
* **gross_risk_pct** — gross risk exposure = the sum of all non-reserve weights
  (percentage points). The complement to ``max_risk`` is the reserve.
* **hhi** — the Herfindahl-Hirschman Index of the *risk* book: the sum of each
  position's weight-fraction (of gross risk) squared. ``1.0`` means everything
  is in one name; lower is more diversified.
* **effective_positions** — ``1 / hhi``, the "effective number of positions";
  the diversification-equivalent count of equally-weighted bets.

Between the blend and each single skill it also reports overlap /
diversification:

* **shared_symbols** — how many risk symbols both books hold.
* **overlap_pct** — the portfolio-overlap score, ``Σ min(w_blend, w_skill)``
  over the union of risk symbols, expressed as a fraction of the *smaller*
  book's gross risk (so a single skill fully contained in the blend scores
  100%). This is a standard book-similarity measure.
* **jaccard** — Jaccard index over the two symbol *sets* (intersection / union).

Design contract (mirrors :mod:`guardrail_lab.ensemble`):

* Standard-library only; all public functions are pure and return frozen
  dataclasses without mutating their inputs.
* Nothing raises on missing/malformed files. A missing config, an unknown
  regime, or absent skill examples yields a clearly-empty
  :class:`RegimeComparison` carrying a human-readable ``reason``, and
  :func:`render_markdown` renders that reason rather than raising.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .ensemble import (
    DEFAULT_CONFIG_PATH,
    DEFAULT_RESERVE_SYMBOL,
    DEFAULT_SKILLS_ROOT,
    KNOWN_REGIMES,
    EnsembleResult,
    blend_regime,
    load_config,
    load_skill_portfolio,
)

#: Label used for the blended ensemble book in rendered output.
BLEND_LABEL = "ENSEMBLE (blend)"


@dataclass(frozen=True)
class BookShape:
    """Defensible portfolio-shape metrics for a single risk book.

    A "book" is either the blended ensemble target or one single skill's
    example target portfolio, both for the *same* regime. Only the risk
    (non-reserve) positions feed the concentration metrics; the reserve symbol
    is reported separately because it is a capital-preservation line, not a bet.

    Attributes:
        name: Human-readable book name (a skill name, or :data:`BLEND_LABEL`).
        loaded: ``True`` when a non-empty book was available for this regime.
        n_positions: Number of distinct risk (non-reserve) positions.
        gross_risk_pct: Gross risk exposure — sum of non-reserve weights (pp).
        reserve_pct: Weight held in the reserve symbol (pp).
        hhi: Herfindahl index of the risk book (sum of weight-fractions^2), in
            ``(0, 1]``; ``0.0`` when there are no risk positions.
        effective_positions: ``1 / hhi`` (the diversification-equivalent count);
            ``0.0`` when ``hhi`` is ``0``.
        top_symbol: The largest risk position's symbol (empty when none).
        top_weight_pct: The largest risk position's weight (pp).
    """

    name: str
    loaded: bool
    n_positions: int = 0
    gross_risk_pct: float = 0.0
    reserve_pct: float = 0.0
    hhi: float = 0.0
    effective_positions: float = 0.0
    top_symbol: str = ""
    top_weight_pct: float = 0.0


@dataclass(frozen=True)
class SkillComparison:
    """The blend-vs-single-skill comparison for one skill in one regime.

    Attributes:
        skill: The single skill's directory name.
        shape: The single skill's own :class:`BookShape` for this regime.
        shared_symbols: Count of risk symbols held by both the blend and skill.
        union_symbols: Count of risk symbols held by either book.
        overlap_pct: ``Σ min(w_blend, w_skill)`` over the union of risk symbols,
            as a fraction (0..1) of the *smaller* book's gross risk.
        jaccard: Jaccard index over the two risk-symbol sets (0..1).
        diversification_delta: ``blend.effective_positions -
            skill.effective_positions`` — positive means the blend is more
            diversified (more effective positions) than this skill alone.
    """

    skill: str
    shape: BookShape
    shared_symbols: int = 0
    union_symbols: int = 0
    overlap_pct: float = 0.0
    jaccard: float = 0.0
    diversification_delta: float = 0.0


@dataclass(frozen=True)
class RegimeComparison:
    """The full ensemble-vs-skills comparison for a single regime.

    Attributes:
        regime: The regime this comparison was computed for.
        ok: ``True`` when the blend produced a non-empty book and at least one
            single skill book was available to compare against.
        reason: Human-readable explanation (empty on success; populated when
            ``ok`` is ``False`` or some skills were skipped).
        reserve_symbol: The reserve / quote symbol excluded from risk metrics.
        blend: The blended ensemble book's :class:`BookShape`.
        skills: Per-skill comparisons, in config order.
    """

    regime: str
    ok: bool
    reason: str
    reserve_symbol: str = DEFAULT_RESERVE_SYMBOL
    blend: BookShape = field(
        default_factory=lambda: BookShape(name=BLEND_LABEL, loaded=False)
    )
    skills: list[SkillComparison] = field(default_factory=list)


def _risk_weights(
    book: dict[str, float], reserve_symbol: str
) -> dict[str, float]:
    """Drop the reserve symbol and any non-positive weights from a book."""
    return {
        symbol: weight
        for symbol, weight in book.items()
        if symbol != reserve_symbol and weight > 0
    }


def _shape_from_weights(
    name: str,
    risk: dict[str, float],
    reserve_pct: float,
    loaded: bool,
) -> BookShape:
    """Compute a :class:`BookShape` from a ``{symbol: weight_pct}`` risk book."""
    gross = sum(risk.values())
    if not risk or gross <= 0:
        return BookShape(
            name=name,
            loaded=loaded,
            n_positions=0,
            gross_risk_pct=round(gross, 4),
            reserve_pct=round(max(0.0, reserve_pct), 4),
        )

    # Fractions of gross risk; HHI = Σ fraction^2 (in (0, 1]).
    hhi = sum((weight / gross) ** 2 for weight in risk.values())
    effective = (1.0 / hhi) if hhi > 0 else 0.0
    top_symbol, top_weight = max(
        risk.items(), key=lambda item: (item[1], item[0])
    )

    return BookShape(
        name=name,
        loaded=loaded,
        n_positions=len(risk),
        gross_risk_pct=round(gross, 4),
        reserve_pct=round(max(0.0, reserve_pct), 4),
        hhi=round(hhi, 6),
        effective_positions=round(effective, 4),
        top_symbol=top_symbol,
        top_weight_pct=round(top_weight, 4),
    )


def _blend_book(result: EnsembleResult, reserve_symbol: str) -> dict[str, float]:
    """Extract the ``{symbol: weight_pct}`` risk book from an EnsembleResult."""
    return {
        position.symbol: position.weight_pct
        for position in result.target_portfolio
        if position.symbol != reserve_symbol and position.weight_pct > 0
    }


def _compare_books(
    blend_risk: dict[str, float],
    skill_risk: dict[str, float],
    blend_effective: float,
    skill_shape: BookShape,
    skill_name: str,
) -> SkillComparison:
    """Compute overlap / diversification of one skill book vs the blend book."""
    blend_symbols = set(blend_risk)
    skill_symbols = set(skill_risk)
    shared = blend_symbols & skill_symbols
    union = blend_symbols | skill_symbols

    # Portfolio overlap: Σ min(weight) over the union, normalized by the
    # smaller book's gross risk so a fully-contained book scores 100%.
    overlap_mass = sum(
        min(blend_risk.get(symbol, 0.0), skill_risk.get(symbol, 0.0))
        for symbol in union
    )
    blend_gross = sum(blend_risk.values())
    skill_gross = sum(skill_risk.values())
    denom = min(blend_gross, skill_gross)
    overlap_pct = (overlap_mass / denom) if denom > 0 else 0.0

    jaccard = (len(shared) / len(union)) if union else 0.0

    return SkillComparison(
        skill=skill_name,
        shape=skill_shape,
        shared_symbols=len(shared),
        union_symbols=len(union),
        overlap_pct=round(overlap_pct, 6),
        jaccard=round(jaccard, 6),
        diversification_delta=round(
            blend_effective - skill_shape.effective_positions, 4
        ),
    )


def compare_regime(
    regime: str,
    config_path: str = DEFAULT_CONFIG_PATH,
    skills_root: str = DEFAULT_SKILLS_ROOT,
) -> RegimeComparison:
    """Compare the blended ensemble book against each single skill for ``regime``.

    Reuses :func:`guardrail_lab.ensemble.blend_regime` for the blended book and
    :func:`guardrail_lab.ensemble.load_skill_portfolio` for each single skill's
    own example book, then derives diversification / overlap metrics. Never
    raises: a missing config, unknown regime, or empty blend yields a
    :class:`RegimeComparison` with ``ok=False`` and a populated ``reason``.

    Args:
        regime: One of :data:`guardrail_lab.ensemble.KNOWN_REGIMES`.
        config_path: Path to ``skills/ensemble.json``.
        skills_root: Root directory holding the skill directories.

    Returns:
        A :class:`RegimeComparison`.
    """
    blend_result = blend_regime(
        regime, config_path=config_path, skills_root=skills_root
    )
    reserve_symbol = blend_result.reserve_symbol or DEFAULT_RESERVE_SYMBOL

    if not blend_result.ok:
        return RegimeComparison(
            regime=regime,
            ok=False,
            reason=(
                "no blended book to compare against: " + blend_result.reason
            ),
            reserve_symbol=reserve_symbol,
            blend=BookShape(name=BLEND_LABEL, loaded=False),
        )

    blend_risk = _blend_book(blend_result, reserve_symbol)
    blend_shape = _shape_from_weights(
        BLEND_LABEL, blend_risk, blend_result.reserve_pct, loaded=True
    )

    # The set of skills to compare comes from the config's regime weights, so we
    # compare exactly the skills that contributed to (or were eligible for) the
    # blend. Fall back to the contribution list when the config can't be read.
    skill_names = _skill_names_for_regime(regime, config_path, blend_result)

    comparisons: list[SkillComparison] = []
    skipped: list[str] = []
    for skill in skill_names:
        portfolio = load_skill_portfolio(skill, regime, skills_root)
        skill_risk = _risk_weights(portfolio, reserve_symbol)
        loaded = bool(portfolio)
        if not loaded or not skill_risk:
            skipped.append(skill)
        reserve_in_book = portfolio.get(reserve_symbol, 0.0)
        skill_shape = _shape_from_weights(
            skill, skill_risk, reserve_in_book, loaded=loaded
        )
        comparisons.append(
            _compare_books(
                blend_risk,
                skill_risk,
                blend_shape.effective_positions,
                skill_shape,
                skill,
            )
        )

    loaded_any = any(c.shape.loaded and c.shape.n_positions for c in comparisons)
    reason = ""
    if not loaded_any:
        reason = (
            f"blended book available but no single-skill book could be loaded "
            f"for regime '{regime}' under '{skills_root}'."
        )
    elif skipped:
        reason = "compared with partial inputs; skipped: " + ", ".join(skipped)

    return RegimeComparison(
        regime=regime,
        ok=loaded_any,
        reason=reason,
        reserve_symbol=reserve_symbol,
        blend=blend_shape,
        skills=comparisons,
    )


def _skill_names_for_regime(
    regime: str,
    config_path: str,
    blend_result: EnsembleResult,
) -> list[str]:
    """Resolve the ordered list of skills to compare for a regime.

    Prefers the config's per-regime weight keys (sorted, matching the blend's
    deterministic order); falls back to the blend result's contribution list so
    the comparison still works if the config layout is unexpected.
    """
    config = load_config(config_path)
    if config is not None:
        regime_cfg = config["regimes"].get(regime)
        if isinstance(regime_cfg, dict) and isinstance(
            regime_cfg.get("weights"), dict
        ):
            return sorted(regime_cfg["weights"])
    return [c.skill for c in blend_result.contributions]


def compare_all(
    config_path: str = DEFAULT_CONFIG_PATH,
    skills_root: str = DEFAULT_SKILLS_ROOT,
    regimes: tuple[str, ...] = KNOWN_REGIMES,
) -> list[RegimeComparison]:
    """Compare every regime in ``regimes`` (defaults to all four known regimes).

    Never raises; each regime is compared independently so one missing regime
    does not block the others.
    """
    return [
        compare_regime(regime, config_path=config_path, skills_root=skills_root)
        for regime in regimes
    ]


def _render_shape_row(shape: BookShape) -> str:
    """Render one :class:`BookShape` as a Markdown table row."""
    if not shape.loaded or shape.n_positions == 0:
        return (
            f"| {shape.name} | n/a | n/a | n/a | n/a | n/a | "
            f"{shape.reserve_pct:.2f} |"
        )
    top = f"{shape.top_symbol} ({shape.top_weight_pct:.2f})"
    return (
        f"| {shape.name} | {shape.n_positions} | "
        f"{shape.gross_risk_pct:.2f} | {shape.hhi:.4f} | "
        f"{shape.effective_positions:.2f} | {top} | "
        f"{shape.reserve_pct:.2f} |"
    )


def render_markdown(comparison: RegimeComparison) -> str:
    """Render a :class:`RegimeComparison` as readable Markdown.

    Always returns a non-empty string; on a failed/empty comparison it renders
    the ``reason`` rather than raising.

    Args:
        comparison: The comparison to render.

    Returns:
        A Markdown report string.
    """
    lines: list[str] = []
    lines.append(f"# Ensemble vs. Single Skills — regime `{comparison.regime}`")
    lines.append("")

    if not comparison.ok:
        lines.append(f"**No comparison produced.** {comparison.reason}")
        lines.append("")
        return "\n".join(lines)

    if comparison.reason:
        lines.append(f"_Note: {comparison.reason}_")
        lines.append("")

    lines.append(
        "Concentration / diversification of each risk book "
        f"(reserve symbol `{comparison.reserve_symbol}` excluded from risk "
        "metrics). HHI is the Herfindahl index of risk weight-fractions "
        "(lower = more diversified); effective positions is 1/HHI."
    )
    lines.append("")

    lines.append("## Book Shape")
    lines.append("")
    lines.append(
        "| Book | # risk pos | Gross risk % | HHI | Eff. positions "
        "| Top position | Reserve % |"
    )
    lines.append("| --- | ---: | ---: | ---: | ---: | --- | ---: |")
    lines.append(_render_shape_row(comparison.blend))
    for skill in comparison.skills:
        lines.append(_render_shape_row(skill.shape))
    lines.append("")

    lines.append("## Blend vs. Each Skill — Overlap & Diversification")
    lines.append("")
    lines.append(
        "| Skill | Shared / union | Overlap % | Jaccard "
        "| Δ eff. positions vs blend |"
    )
    lines.append("| --- | :---: | ---: | ---: | ---: |")
    for skill in comparison.skills:
        if not skill.shape.loaded or skill.shape.n_positions == 0:
            lines.append(
                f"| {skill.skill} | n/a | n/a | n/a | n/a |"
            )
            continue
        delta = skill.diversification_delta
        sign = "+" if delta >= 0 else ""
        lines.append(
            f"| {skill.skill} | {skill.shared_symbols}/{skill.union_symbols} "
            f"| {skill.overlap_pct * 100:.1f} | {skill.jaccard:.3f} "
            f"| {sign}{delta:.2f} |"
        )
    lines.append("")

    lines.append(
        "_A positive Δ eff. positions means the blended book is more "
        "diversified (more effective positions) than that skill alone. Overlap "
        "% is the share of the smaller book's gross risk that the two books "
        "hold in common. The blended book is advisory; the Rust risk engine "
        "remains the sole execution gate._"
    )
    lines.append("")
    return "\n".join(lines)


def render_markdown_all(comparisons: list[RegimeComparison]) -> str:
    """Render a list of regime comparisons as one Markdown document.

    Args:
        comparisons: The per-regime comparisons (e.g. from :func:`compare_all`).

    Returns:
        A single Markdown string joining each regime section with a horizontal
        rule. Returns a clear note when the list is empty.
    """
    if not comparisons:
        return (
            "# Ensemble vs. Single Skills\n\n"
            "_No regimes to compare._\n"
        )
    sections = [render_markdown(comparison) for comparison in comparisons]
    return "\n---\n\n".join(sections)
