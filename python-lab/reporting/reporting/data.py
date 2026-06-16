"""Data readers for the Guardrail SQLite event log and JSON run report.

The event log lives in ``data/guardrail_alpha.db`` and has a single table::

    events(id TEXT, run_id TEXT, timestamp TEXT, event_type TEXT, payload_json TEXT)

The run report lives in ``data/run_report.json`` and is a flat object with
fields such as ``nav_usd``, ``starting_nav_usd``, ``positions`` and ``trades``.

All readers validate at the boundary and fail fast with clear messages, and
return immutable-friendly dataclasses / tuples rather than mutating inputs.
"""

from __future__ import annotations

import json
import os
import sqlite3
from dataclasses import dataclass, field, replace
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from typing import Any, Mapping, Optional, Sequence


# --------------------------------------------------------------------------- #
# Parsing helpers
# --------------------------------------------------------------------------- #

def _parse_timestamp(value: str) -> Optional[datetime]:
    """Parse an ISO-8601 timestamp, tolerating a trailing 'Z'.

    Returns a timezone-aware datetime in UTC, or ``None`` if unparseable.
    """
    if not value:
        return None
    raw = value.strip()
    if raw.endswith("Z"):
        raw = raw[:-1] + "+00:00"
    try:
        dt = datetime.fromisoformat(raw)
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


def _to_decimal(value: Any) -> Optional[Decimal]:
    """Best-effort conversion of an arbitrary JSON value to Decimal."""
    if value is None:
        return None
    if isinstance(value, Decimal):
        return value
    try:
        # str() first so floats are read via their repr, ints/strings pass through.
        return Decimal(str(value))
    except (InvalidOperation, ValueError, TypeError):
        return None


# --------------------------------------------------------------------------- #
# Event log
# --------------------------------------------------------------------------- #

@dataclass(frozen=True)
class Event:
    """A single decoded row from the events table."""

    id: str
    run_id: str
    timestamp: Optional[datetime]
    timestamp_raw: str
    event_type: str
    payload: Mapping[str, Any]


@dataclass(frozen=True)
class NavPoint:
    """One observation of NAV over time."""

    timestamp: Optional[datetime]
    timestamp_raw: str
    nav: Decimal


@dataclass(frozen=True)
class ConfirmedTrade:
    """A confirmed transaction from a ``tx_confirmed`` event."""

    timestamp: Optional[datetime]
    timestamp_raw: str
    tx_hash: Optional[str]
    competition_tx: Optional[str]
    block: Optional[int]
    status: Optional[str]


@dataclass(frozen=True)
class EventLog:
    """The fully-decoded event log for a single database file."""

    db_path: str
    run_ids: Sequence[str]
    event_counts: Mapping[str, int]
    total_events: int
    nav_series: Sequence[NavPoint]
    confirmed_trades: Sequence[ConfirmedTrade]

    @property
    def event_types(self) -> Sequence[str]:
        return tuple(sorted(self.event_counts))


def load_event_log(db_path: str, run_id: Optional[str] = None) -> EventLog:
    """Read and decode the event log from a SQLite database.

    Args:
        db_path: Path to the SQLite database file.
        run_id: Optional run_id filter. When given, only events for that run
            are considered.

    Raises:
        FileNotFoundError: if the database file does not exist.
        ValueError: if the expected ``events`` table is missing.
    """
    if not os.path.isfile(db_path):
        raise FileNotFoundError(f"event database not found: {db_path}")

    # Open read-only so we never accidentally mutate the agent's log.
    uri = f"file:{os.path.abspath(db_path)}?mode=ro"
    con = sqlite3.connect(uri, uri=True)
    try:
        con.row_factory = sqlite3.Row
        cur = con.cursor()

        tables = {
            r[0]
            for r in cur.execute(
                "SELECT name FROM sqlite_master WHERE type='table'"
            )
        }
        if "events" not in tables:
            raise ValueError(
                f"database {db_path!r} has no 'events' table (found: {sorted(tables)})"
            )

        where = ""
        params: list[Any] = []
        if run_id:
            where = "WHERE run_id = ?"
            params.append(run_id)

        rows = list(
            cur.execute(
                f"SELECT id, run_id, timestamp, event_type, payload_json "
                f"FROM events {where} ORDER BY timestamp ASC, id ASC",
                params,
            )
        )
    finally:
        con.close()

    events = _decode_rows(rows)

    event_counts: dict[str, int] = {}
    run_ids: list[str] = []
    nav_series: list[NavPoint] = []
    confirmed_trades: list[ConfirmedTrade] = []

    for ev in events:
        event_counts[ev.event_type] = event_counts.get(ev.event_type, 0) + 1
        if ev.run_id and ev.run_id not in run_ids:
            run_ids.append(ev.run_id)

        if ev.event_type == "portfolio_reconciled":
            nav = _to_decimal(ev.payload.get("nav_usd"))
            if nav is not None:
                nav_series.append(
                    NavPoint(
                        timestamp=ev.timestamp,
                        timestamp_raw=ev.timestamp_raw,
                        nav=nav,
                    )
                )
        elif ev.event_type == "tx_confirmed":
            p = ev.payload
            block = p.get("block")
            try:
                block_int = int(block) if block is not None else None
            except (TypeError, ValueError):
                block_int = None
            confirmed_trades.append(
                ConfirmedTrade(
                    timestamp=ev.timestamp,
                    timestamp_raw=ev.timestamp_raw,
                    tx_hash=p.get("tx_hash"),
                    competition_tx=p.get("competition_tx"),
                    block=block_int,
                    status=p.get("status"),
                )
            )

    return EventLog(
        db_path=os.path.abspath(db_path),
        run_ids=tuple(run_ids),
        event_counts=event_counts,
        total_events=len(events),
        nav_series=tuple(nav_series),
        confirmed_trades=tuple(confirmed_trades),
    )


def _decode_rows(rows: Sequence[sqlite3.Row]) -> list[Event]:
    """Decode raw DB rows into Event objects, tolerating bad payload JSON."""
    decoded: list[Event] = []
    for r in rows:
        raw_payload = r["payload_json"]
        try:
            payload = json.loads(raw_payload) if raw_payload else {}
        except (json.JSONDecodeError, TypeError):
            payload = {}
        if not isinstance(payload, dict):
            payload = {"value": payload}
        ts_raw = r["timestamp"] or ""
        decoded.append(
            Event(
                id=str(r["id"]) if r["id"] is not None else "",
                run_id=str(r["run_id"]) if r["run_id"] is not None else "",
                timestamp=_parse_timestamp(ts_raw),
                timestamp_raw=ts_raw,
                event_type=str(r["event_type"]) if r["event_type"] is not None else "",
                payload=payload,
            )
        )
    return decoded


# --------------------------------------------------------------------------- #
# Run report
# --------------------------------------------------------------------------- #

@dataclass(frozen=True)
class Position:
    symbol: str
    value_usd: Optional[Decimal]
    weight_pct: Optional[Decimal]


@dataclass(frozen=True)
class RunReport:
    """Decoded view of ``run_report.json``.

    The ``raw`` mapping preserves every field for display; the typed fields are
    convenience accessors for the values the report renders prominently.
    """

    path: str
    run_id: Optional[str]
    agent_id: Optional[str]
    mode: Optional[str]
    regime: Optional[str]
    kill_switch: Optional[bool]
    nav_usd: Optional[Decimal]
    starting_nav_usd: Optional[Decimal]
    total_drawdown_pct: Optional[Decimal]
    positions: Sequence[Position]
    trades: Sequence[Mapping[str, Any]]
    event_count: Optional[int]
    raw: Mapping[str, Any] = field(default_factory=dict)


def load_run_report(path: Optional[str]) -> Optional[RunReport]:
    """Read and decode the JSON run report.

    A missing path (``None``) returns ``None`` so the report can still render
    from the event log alone. A path that is supplied but missing on disk, or
    that contains invalid JSON, raises so the caller learns of the mistake.
    """
    if path is None:
        return None
    if not os.path.isfile(path):
        raise FileNotFoundError(f"run report not found: {path}")

    with open(path, "r", encoding="utf-8") as fh:
        try:
            data = json.load(fh)
        except json.JSONDecodeError as exc:
            raise ValueError(f"run report {path!r} is not valid JSON: {exc}") from exc

    if not isinstance(data, dict):
        raise ValueError(f"run report {path!r} must be a JSON object")

    positions: list[Position] = []
    for item in data.get("positions", []) or []:
        if not isinstance(item, dict):
            continue
        positions.append(
            Position(
                symbol=str(item.get("symbol", "")),
                value_usd=_to_decimal(item.get("value_usd")),
                weight_pct=_to_decimal(item.get("weight_pct")),
            )
        )

    trades = [t for t in (data.get("trades") or []) if isinstance(t, dict)]

    kill = data.get("kill_switch")
    if not isinstance(kill, bool):
        kill = None

    event_count = data.get("events")
    try:
        event_count = int(event_count) if event_count is not None else None
    except (TypeError, ValueError):
        event_count = None

    return RunReport(
        path=os.path.abspath(path),
        run_id=data.get("run_id"),
        agent_id=data.get("agent_id"),
        mode=data.get("mode"),
        regime=data.get("regime"),
        kill_switch=kill,
        nav_usd=_to_decimal(data.get("nav_usd")),
        starting_nav_usd=_to_decimal(data.get("starting_nav_usd")),
        total_drawdown_pct=_to_decimal(data.get("total_drawdown_pct")),
        positions=tuple(positions),
        trades=tuple(trades),
        event_count=event_count,
        raw=dict(data),
    )


def with_run_id(report: RunReport, run_id: str) -> RunReport:
    """Return a copy of the report with a different run_id (immutable update)."""
    return replace(report, run_id=run_id)
