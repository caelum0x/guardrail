"""Read-only command handlers for the Guardrail control bot.

Each command fetches one or more read-only Guardrail API routes and formats a
short chat reply. NONE of these mutate state, sign, or trade — the bot is a
read-only window onto the running agent. Standard library only.
"""

from __future__ import annotations

import json
import urllib.error
import urllib.request
from typing import Callable


def _get(base_url: str, path: str, timeout: float = 5.0) -> dict | list | None:
    """GET a JSON route, returning the decoded body or ``None`` on any failure."""
    url = f"{base_url.rstrip('/')}{path}"
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:  # noqa: S310 (trusted base)
            return json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, OSError, ValueError, TimeoutError):
        return None


def _fmt_status(base_url: str) -> str:
    compete = _get(base_url, "/compete")
    readiness = _get(base_url, "/readiness")
    if compete is None and readiness is None:
        return "status: API unreachable"
    lines = ["*Guardrail status*"]
    if isinstance(compete, dict):
        lines.append(f"  registered: {compete.get('registered', '?')}")
        lines.append(f"  eligible assets: {compete.get('eligible_assets', '?')}")
        lines.append(f"  confirmed trades: {compete.get('confirmed_trades', '?')}")
        lines.append(f"  kill switch: {compete.get('kill_switch', '?')}")
    if isinstance(readiness, dict):
        lines.append(f"  readiness: {readiness.get('status', '?')}")
    return "\n".join(lines)


def _fmt_regime(base_url: str) -> str:
    regime = _get(base_url, "/regime")
    if not isinstance(regime, dict):
        return "regime: unavailable"
    return (
        f"*Regime*: {regime.get('regime', '?')} "
        f"(exposure x{regime.get('exposure_multiplier', '?')})"
    )


def _fmt_journal(base_url: str) -> str:
    journal = _get(base_url, "/journal")
    if not isinstance(journal, dict):
        return "journal: unavailable"
    cycles = journal.get("cycles") or journal.get("entries") or []
    n = len(cycles) if isinstance(cycles, list) else "?"
    return f"*Journal*: {n} cycle(s) recorded"


def _fmt_verify(base_url: str) -> str:
    verify = _get(base_url, "/proof/verify")
    if not isinstance(verify, dict):
        return "verify: unavailable"
    checks = verify.get("checks") or []
    passed = verify.get("passed", "?")
    return f"*Proof verify*: passed={passed} ({len(checks)} checks)"


def _fmt_skills(base_url: str) -> str:
    skills = _get(base_url, "/skills")
    if not isinstance(skills, dict):
        return "skills: unavailable"
    names = [s.get("name", s.get("id", "?")) for s in skills.get("skills", [])]
    return f"*Skills* ({skills.get('count', len(names))}): " + ", ".join(names)


# Public command registry: name -> formatter(base_url) -> reply text.
COMMANDS: dict[str, Callable[[str], str]] = {
    "status": _fmt_status,
    "regime": _fmt_regime,
    "journal": _fmt_journal,
    "verify": _fmt_verify,
    "skills": _fmt_skills,
}


def answer(base_url: str, command: str) -> str:
    """Produce the read-only reply for a command, or a help line if unknown."""
    handler = COMMANDS.get(command.lstrip("/").strip().lower())
    if handler is None:
        return "commands: " + ", ".join(f"/{c}" for c in COMMANDS)
    return handler(base_url)
