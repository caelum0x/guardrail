"""Read an equity/NAV series from the Guardrail SQLite event log.

The live agent appends `portfolio_reconciled` events whose payload carries
`nav_usd`; in chronological order these form the equity curve. Read-only.
"""

from __future__ import annotations

import json
import os
import sqlite3

DEFAULT_DB = "data/guardrail_alpha.db"


def equity_series(db_path: str = DEFAULT_DB) -> list[float]:
    """Return the NAV series (oldest first) from the event log, or [] if absent."""
    if not os.path.isfile(db_path):
        return []
    try:
        conn = sqlite3.connect(db_path)
        rows = conn.execute(
            "SELECT payload_json FROM events "
            "WHERE event_type = 'portfolio_reconciled' ORDER BY timestamp ASC, id ASC"
        ).fetchall()
        conn.close()
    except sqlite3.Error:
        return []
    out: list[float] = []
    for (payload,) in rows:
        try:
            nav = json.loads(payload).get("nav_usd")
            if nav is not None:
                out.append(float(nav))
        except (json.JSONDecodeError, TypeError, ValueError):
            continue
    return out
