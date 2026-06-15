"""SQLite event-log access for Guardrail Alpha.

Standard-library only (sqlite3, json, pathlib) so it runs without pip installs.
"""

import json
import sqlite3
from pathlib import Path


def database_path() -> Path:
    """Default location of the event-log database (relative to repo root)."""
    return Path("data/guardrail_alpha.db")


def load_events(db_path: str = "data/guardrail_alpha.db") -> list[dict]:
    """Read every row from the ``events`` table, ordered by timestamp.

    The ``payload_json`` column is parsed into a ``payload`` dict. Rows whose
    payload cannot be parsed keep an empty payload rather than failing the run.
    Returns an empty list when the database file does not exist.
    """
    path = Path(db_path)
    if not path.exists():
        return []

    events: list[dict] = []
    connection = sqlite3.connect(str(path))
    try:
        connection.row_factory = sqlite3.Row
        cursor = connection.execute(
            "SELECT id, run_id, timestamp, event_type, payload_json "
            "FROM events ORDER BY timestamp ASC, id ASC"
        )
        for row in cursor:
            raw_payload = row["payload_json"]
            try:
                payload = json.loads(raw_payload) if raw_payload else {}
            except (json.JSONDecodeError, TypeError):
                payload = {}
            events.append(
                {
                    "id": row["id"],
                    "run_id": row["run_id"],
                    "timestamp": row["timestamp"],
                    "event_type": row["event_type"],
                    "payload": payload,
                }
            )
    finally:
        connection.close()

    return events


def event_counts(events: list[dict]) -> dict:
    """Count events by ``event_type``, returning a dict sorted by type name."""
    counts: dict[str, int] = {}
    for event in events:
        event_type = event.get("event_type", "unknown")
        counts[event_type] = counts.get(event_type, 0) + 1
    return dict(sorted(counts.items()))
