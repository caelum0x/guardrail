"""Trade attribution and regime analysis derived from the event log.

Standard-library only. All functions tolerate missing or malformed payload
fields rather than raising, so they are safe to run against partial event logs.

Event model (per cycle, in timestamp order)::

    regime_classified  -> {"regime": "risk_on"}
    order_proposed     -> {"from": "USDT", "to": "CAKE", "amount_usd": "1700.00"}
    twak_quote_received
    risk_approved | risk_clipped -> {"amount_usd": "<final>"}   (optional)
    twak_swap_submitted
    tx_confirmed       -> {"tx_hash": ..., "status": "confirmed", ...}

A confirmed swap (``tx_confirmed``) carries no destination symbol of its own,
so it is correlated with the most recent preceding ``order_proposed`` (for the
``to`` symbol and fallback amount) and, when present, the most recent
``risk_approved``/``risk_clipped`` decision (for the final executed amount).
"""

ORDER_EVENT = "order_proposed"
RISK_APPROVED_EVENT = "risk_approved"
RISK_CLIPPED_EVENT = "risk_clipped"
CONFIRMED_EVENT = "tx_confirmed"
REGIME_EVENT = "regime_classified"

UNKNOWN_SYMBOL = "UNKNOWN"


def _to_float(value: object) -> float | None:
    """Best-effort conversion of a payload value (often a decimal string)."""
    if value is None:
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


def _payload(event: dict) -> dict:
    """Return the event payload as a dict, never ``None``."""
    payload = event.get("payload")
    return payload if isinstance(payload, dict) else {}


def trade_attribution(events: list[dict]) -> list[dict]:
    """Summarize confirmed swaps grouped by destination symbol.

    Walks the event log in order, tracking the latest ``order_proposed`` (for
    the destination symbol and a fallback amount) and the latest risk decision
    (``risk_approved`` / ``risk_clipped``, for the final executed amount). When
    a ``tx_confirmed`` event is seen the swap is attributed to the pending
    order's destination symbol.

    Returns a list of dicts, sorted by descending ``total_amount_usd`` then
    symbol, each shaped as::

        {
            "symbol": "CAKE",
            "count": 2,
            "total_amount_usd": 1700.85,
        }

    Missing or unparseable fields degrade gracefully: an unknown destination is
    reported under ``"UNKNOWN"`` and an unparseable amount contributes ``0.0``.
    """
    if not events:
        return []

    ordered = sorted(
        events,
        key=lambda event: (event.get("timestamp") or "", event.get("id") or ""),
    )

    summaries: dict[str, dict] = {}
    pending_order: dict | None = None
    pending_risk_amount: float | None = None

    for event in ordered:
        event_type = event.get("event_type")
        payload = _payload(event)

        if event_type == ORDER_EVENT:
            pending_order = payload
            pending_risk_amount = None
        elif event_type in (RISK_APPROVED_EVENT, RISK_CLIPPED_EVENT):
            pending_risk_amount = _to_float(payload.get("amount_usd"))
        elif event_type == CONFIRMED_EVENT:
            symbol = UNKNOWN_SYMBOL
            order_amount: float | None = None
            if pending_order is not None:
                raw_symbol = pending_order.get("to")
                if isinstance(raw_symbol, str) and raw_symbol.strip():
                    symbol = raw_symbol.strip()
                order_amount = _to_float(pending_order.get("amount_usd"))

            # Prefer the risk-adjusted amount; fall back to the proposed order
            # amount; finally treat an unknown amount as zero.
            amount = pending_risk_amount
            if amount is None:
                amount = order_amount
            if amount is None:
                amount = 0.0

            summary = summaries.get(symbol)
            if summary is None:
                summary = {"symbol": symbol, "count": 0, "total_amount_usd": 0.0}
                summaries[symbol] = summary
            summary["count"] += 1
            summary["total_amount_usd"] += amount

            # Reset so a later confirm without a fresh order is not
            # mis-attributed to this one.
            pending_order = None
            pending_risk_amount = None

    results = list(summaries.values())
    for summary in results:
        summary["total_amount_usd"] = round(summary["total_amount_usd"], 2)

    results.sort(key=lambda item: (-item["total_amount_usd"], item["symbol"]))
    return results


def regime_timeline(events: list[dict]) -> list[dict]:
    """List ``(timestamp, regime)`` pairs from ``regime_classified`` events.

    Returns a list of dicts ``{"timestamp": ..., "regime": ...}`` ordered by
    timestamp. Events missing a regime label are reported with ``"unknown"``;
    events missing a timestamp keep an empty string so they still appear.
    """
    timeline: list[dict] = []
    for event in events:
        if event.get("event_type") != REGIME_EVENT:
            continue
        payload = _payload(event)
        raw_regime = payload.get("regime")
        if isinstance(raw_regime, str) and raw_regime.strip():
            regime = raw_regime.strip()
        else:
            regime = "unknown"
        timeline.append(
            {
                "timestamp": event.get("timestamp") or "",
                "regime": regime,
            }
        )

    timeline.sort(key=lambda item: item["timestamp"])
    return timeline
