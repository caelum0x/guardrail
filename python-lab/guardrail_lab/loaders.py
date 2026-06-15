"""Loaders for the Rust agent's JSON run report.

Standard-library only (json, pathlib).
"""

import json
from pathlib import Path


def load_run_report(path: str = "data/run_report.json") -> dict | None:
    """Load ``run_report.json`` produced by the agent.

    Returns the parsed dict, or ``None`` when the file is missing or the
    contents cannot be parsed as a JSON object.
    """
    report_path = Path(path)
    if not report_path.exists():
        return None

    try:
        with report_path.open("r", encoding="utf-8") as handle:
            data = json.load(handle)
    except (json.JSONDecodeError, OSError):
        return None

    if not isinstance(data, dict):
        return None

    return data
