"""Human-readable decision journal built from the append-only event log.

Guardrail Alpha emits, per decision cycle, an ordered stream of events:

    agent_started            -> {"agent_id": ..., "mode": "paper", ...}
    market_snapshot_received -> {"assets": 20, "ts": ...}
    regime_classified        -> {"regime": "risk_on"}
    portfolio_target_computed-> {"headline": ..., "orders": 5, ...}
    asset_scored             -> {"symbol": "SHIB", "score": 0.718}
    order_proposed           -> {"from": "USDT", "to": "SHIB", "amount_usd": ...}
    twak_quote_received      -> {"route": "q_2", "slippage_pct": "0.0784"}
    risk_approved | risk_clipped | risk_rejected -> {"amount_usd"/"reasons", ...}
    twak_swap_submitted      -> {"amount_usd": ...}
    tx_confirmed             -> {"tx_hash": ..., "status": "confirmed"}
    portfolio_reconciled     -> {"nav_usd": ..., "positions": 5}
    agent_report_published   -> {"final_nav": ..., "cycles": 2, ...}

This module turns that machine log into a narrative a human can read: for each
cycle it tells the story "the agent saw the market -> classified the regime ->
scored assets -> proposed orders -> the risk engine approved/clipped/rejected
-> trades confirmed -> the book was reconciled". This is the verifiable-autonomy
story: every decision is reconstructable from the log alone.

Design contract:

* Standard-library only. All functions are pure: they take the parsed event
  list (from :func:`guardrail_lab.db.load_events`) and return frozen
  dataclasses without touching the filesystem or mutating inputs.
* Degrades gracefully: an empty log yields an empty :class:`Journal`, and
  :func:`render_markdown` renders a clear "no data" note rather than raising.

A *cycle* is a contiguous span of events that begins at a ``regime_classified``
event (the decision point of each trading cycle) and runs until the next
``regime_classified``. Events before the first classification (e.g.
``agent_started``) are attributed to a synthetic "startup" cycle so nothing is
lost.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .db import load_events

REGIME_EVENT = "regime_classified"
SCORE_EVENT = "asset_scored"
ORDER_EVENT = "order_proposed"
TARGET_EVENT = "portfolio_target_computed"
RECONCILE_EVENT = "portfolio_reconciled"
CONFIRM_EVENT = "tx_confirmed"
RISK_APPROVED = "risk_approved"
RISK_CLIPPED = "risk_clipped"
RISK_REJECTED = "risk_rejected"

#: Risk-verdict event types in display order.
_RISK_EVENTS = (RISK_APPROVED, RISK_CLIPPED, RISK_REJECTED)


@dataclass(frozen=True)
class ScoredAsset:
    """A single ``asset_scored`` entry within a cycle."""

    symbol: str
    score: float


@dataclass(frozen=True)
class ProposedOrder:
    """A single ``order_proposed`` entry within a cycle."""

    from_symbol: str
    to_symbol: str
    amount_usd: float | None


@dataclass(frozen=True)
class RiskVerdict:
    """A summary of the risk engine's decisions within a cycle.

    Attributes:
        approved: Count of ``risk_approved`` events.
        clipped: Count of ``risk_clipped`` events.
        rejected: Count of ``risk_rejected`` events.
        rejection_reasons: Flattened, de-duplicated list of human-readable
            rejection reasons pulled from ``risk_rejected`` payloads.
    """

    approved: int = 0
    clipped: int = 0
    rejected: int = 0
    rejection_reasons: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class CycleJournal:
    """The narrative-ready facts of a single decision cycle.

    Attributes:
        index: 1-based cycle number in the run.
        run_id: The originating run identifier (empty if unknown).
        regime: The classified regime (``"startup"`` for the pre-classification
            span, ``"unknown"`` if a classification carried no regime label).
        started_at: Timestamp of the cycle's first event (verbatim).
        ended_at: Timestamp of the cycle's last event (verbatim).
        headline: The ``portfolio_target_computed`` headline, if any.
        top_assets: Scored assets, highest score first.
        orders: Proposed orders in log order.
        risk: The risk-engine verdict summary.
        confirmed_trades: Count of ``tx_confirmed`` events.
        ending_nav: NAV from the last ``portfolio_reconciled`` event, if any.
        positions: Position count from the last reconcile, if any.
    """

    index: int
    run_id: str
    regime: str
    started_at: str
    ended_at: str
    headline: str = ""
    top_assets: list[ScoredAsset] = field(default_factory=list)
    orders: list[ProposedOrder] = field(default_factory=list)
    risk: RiskVerdict = field(default_factory=RiskVerdict)
    confirmed_trades: int = 0
    ending_nav: float | None = None
    positions: int | None = None


@dataclass(frozen=True)
class Journal:
    """A full decision journal: every cycle plus run-level totals.

    Attributes:
        cycles: The per-cycle narratives in chronological order.
        total_events: Number of events the journal was built from.
        run_ids: Distinct run identifiers seen, in first-seen order.
    """

    cycles: list[CycleJournal] = field(default_factory=list)
    total_events: int = 0
    run_ids: list[str] = field(default_factory=list)


def _payload(event: dict) -> dict:
    """Return an event payload as a dict, never ``None``."""
    payload = event.get("payload")
    return payload if isinstance(payload, dict) else {}


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _ordered(events: list[dict]) -> list[dict]:
    """Sort events by ``(timestamp, id)`` without mutating the input."""
    return sorted(
        events,
        key=lambda event: (event.get("timestamp") or "", event.get("id") or 0),
    )


def _segment_cycles(events: list[dict]) -> list[list[dict]]:
    """Split an ordered event list into per-cycle slices at ``regime_classified``.

    Events preceding the first classification form a leading "startup" slice so
    no event is dropped. A log with no classification at all yields a single
    slice containing every event.
    """
    ordered = _ordered(events)
    if not ordered:
        return []

    segments: list[list[dict]] = []
    current: list[dict] = []
    seen_regime = False

    for event in ordered:
        if event.get("event_type") == REGIME_EVENT:
            if current:
                segments.append(current)
            current = [event]
            seen_regime = True
        else:
            current.append(event)
    if current:
        segments.append(current)

    # When there was never a regime classification the single accumulated slice
    # is still valid; nothing special to do. ``seen_regime`` is retained for
    # readability of intent.
    _ = seen_regime
    return segments


def _build_cycle(index: int, slice_events: list[dict]) -> CycleJournal:
    """Construct a :class:`CycleJournal` from one cycle's events."""
    first = slice_events[0]
    last = slice_events[-1]

    regime = "startup"
    if first.get("event_type") == REGIME_EVENT:
        raw = _payload(first).get("regime")
        regime = raw.strip() if isinstance(raw, str) and raw.strip() else "unknown"

    headline = ""
    scored: list[ScoredAsset] = []
    orders: list[ProposedOrder] = []
    approved = clipped = rejected = confirmed = 0
    reasons: list[str] = []
    ending_nav: float | None = None
    positions: int | None = None

    for event in slice_events:
        event_type = event.get("event_type")
        payload = _payload(event)

        if event_type == TARGET_EVENT:
            head = payload.get("headline")
            if isinstance(head, str) and head.strip():
                headline = head.strip()
        elif event_type == SCORE_EVENT:
            symbol = payload.get("symbol")
            score = _to_float(payload.get("score"))
            if isinstance(symbol, str) and symbol.strip() and score is not None:
                scored.append(ScoredAsset(symbol=symbol.strip(), score=score))
        elif event_type == ORDER_EVENT:
            orders.append(
                ProposedOrder(
                    from_symbol=str(payload.get("from") or "?"),
                    to_symbol=str(payload.get("to") or "?"),
                    amount_usd=_to_float(payload.get("amount_usd")),
                )
            )
        elif event_type == RISK_APPROVED:
            approved += 1
        elif event_type == RISK_CLIPPED:
            clipped += 1
        elif event_type == RISK_REJECTED:
            rejected += 1
            raw_reasons = payload.get("reasons")
            if isinstance(raw_reasons, list):
                for reason in raw_reasons:
                    text = str(reason).strip()
                    if text and text not in reasons:
                        reasons.append(text)
        elif event_type == CONFIRM_EVENT:
            confirmed += 1
        elif event_type == RECONCILE_EVENT:
            nav = _to_float(payload.get("nav_usd"))
            if nav is not None:
                ending_nav = nav
            pos = payload.get("positions")
            if isinstance(pos, int):
                positions = pos

    scored.sort(key=lambda asset: (-asset.score, asset.symbol))

    return CycleJournal(
        index=index,
        run_id=str(first.get("run_id") or ""),
        regime=regime,
        started_at=str(first.get("timestamp") or ""),
        ended_at=str(last.get("timestamp") or ""),
        headline=headline,
        top_assets=scored,
        orders=orders,
        risk=RiskVerdict(
            approved=approved,
            clipped=clipped,
            rejected=rejected,
            rejection_reasons=reasons,
        ),
        confirmed_trades=confirmed,
        ending_nav=ending_nav,
        positions=positions,
    )


def build_journal(events: list[dict]) -> Journal:
    """Build the decision :class:`Journal` from a parsed event log.

    Args:
        events: Parsed event log (e.g. from
            :func:`guardrail_lab.db.load_events`).

    Returns:
        A :class:`Journal`. An empty input yields an empty journal (no cycles).
    """
    segments = _segment_cycles(events)
    cycles = [
        _build_cycle(index, slice_events)
        for index, slice_events in enumerate(segments, start=1)
    ]

    run_ids: list[str] = []
    for event in events:
        run_id = event.get("run_id")
        if isinstance(run_id, str) and run_id and run_id not in run_ids:
            run_ids.append(run_id)

    return Journal(cycles=cycles, total_events=len(events), run_ids=run_ids)


def build_journal_from_db(
    db_path: str = "data/guardrail_alpha.db",
) -> Journal:
    """Load events from the database and build the journal.

    Returns an empty :class:`Journal` when the database is missing/empty;
    never raises on missing data.
    """
    return build_journal(load_events(db_path))


def _format_usd(value: float | None) -> str:
    """Render a USD amount, or ``n/a`` when unknown."""
    if value is None:
        return "n/a"
    return f"${value:,.2f}"


def _render_cycle(cycle: CycleJournal, top_n: int) -> list[str]:
    """Render one cycle as Markdown lines."""
    lines: list[str] = []
    title = f"## Cycle {cycle.index} — regime `{cycle.regime}`"
    lines.append(title)
    if cycle.started_at:
        window = cycle.started_at
        if cycle.ended_at and cycle.ended_at != cycle.started_at:
            window = f"{cycle.started_at} → {cycle.ended_at}"
        lines.append(f"_{window}_")
    lines.append("")

    if cycle.headline:
        lines.append(f"**Target:** {cycle.headline}")
        lines.append("")

    # 1. Assets scored.
    if cycle.top_assets:
        shown = cycle.top_assets[:top_n]
        scored = ", ".join(
            f"{asset.symbol} ({asset.score:.3f})" for asset in shown
        )
        more = (
            f" (+{len(cycle.top_assets) - len(shown)} more)"
            if len(cycle.top_assets) > len(shown)
            else ""
        )
        lines.append(f"- **Assets scored:** {scored}{more}")

    # 2. Orders proposed.
    if cycle.orders:
        order_bits = [
            f"{order.from_symbol}→{order.to_symbol} "
            f"({_format_usd(order.amount_usd)})"
            for order in cycle.orders
        ]
        lines.append(f"- **Orders proposed:** " + "; ".join(order_bits))

    # 3. Risk verdict.
    risk = cycle.risk
    if risk.approved or risk.clipped or risk.rejected:
        verdict = (
            f"{risk.approved} approved, {risk.clipped} clipped, "
            f"{risk.rejected} rejected"
        )
        lines.append(f"- **Risk verdict:** {verdict}")
        for reason in risk.rejection_reasons:
            lines.append(f"  - rejected: {reason}")

    # 4. Trades + reconciliation.
    if cycle.confirmed_trades:
        lines.append(f"- **Trades confirmed:** {cycle.confirmed_trades}")
    if cycle.ending_nav is not None or cycle.positions is not None:
        nav = _format_usd(cycle.ending_nav)
        positions = (
            f", {cycle.positions} positions"
            if cycle.positions is not None
            else ""
        )
        lines.append(f"- **Reconciled NAV:** {nav}{positions}")

    if not any(
        (
            cycle.top_assets,
            cycle.orders,
            risk.approved or risk.clipped or risk.rejected,
            cycle.confirmed_trades,
            cycle.ending_nav is not None,
        )
    ):
        lines.append("- (no decision activity recorded in this cycle)")

    lines.append("")
    return lines


def render_markdown(journal: Journal, top_n: int = 5) -> str:
    """Render a :class:`Journal` as a human-readable Markdown narrative.

    Args:
        journal: The journal to render.
        top_n: Maximum number of scored assets to list per cycle.

    Returns:
        A Markdown string. On an empty journal it returns a clear "no data"
        note rather than raising.
    """
    lines: list[str] = ["# Guardrail Alpha — Decision Journal", ""]

    if not journal.cycles:
        lines.append(
            "_No data — the event log is empty. Run the agent first to "
            "generate a decision journal._"
        )
        lines.append("")
        return "\n".join(lines)

    runs = ", ".join(journal.run_ids) if journal.run_ids else "n/a"
    lines.append(
        f"Events: {journal.total_events}  ·  Cycles: {len(journal.cycles)}  ·  "
        f"Run(s): {runs}"
    )
    lines.append("")
    lines.append(
        "Every line below is reconstructed from the append-only event log: "
        "the agent observed the market, classified the regime, scored assets, "
        "proposed orders, the risk engine ruled on each, and trades were "
        "confirmed and reconciled — a fully auditable decision trail."
    )
    lines.append("")

    for cycle in journal.cycles:
        lines.extend(_render_cycle(cycle, top_n))

    return "\n".join(lines)
