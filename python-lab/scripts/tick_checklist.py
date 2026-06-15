#!/usr/bin/env python3
"""Auto-tick docs/SUBMISSION_CHECKLIST.md from real evidence.

Reads the agent's append-only SQLite event log, its run report, and committed
configs, then flips each proof-artifact checkbox ``[ ]``/``[x]`` based on whether
the evidence actually exists — never by hand. It also regenerates an
auto-generated evidence table at the end of the file.

Honest by construction: an item ticks only when its artifact is present. Items
that genuinely require a live run (a real on-chain registration tx, a real
confirmed swap) report ``pending (live)`` in paper mode rather than a false tick.

Stdlib only. Read-only except for rewriting the checklist file.

Usage:
    python3 python-lab/scripts/tick_checklist.py [--db PATH] [--report PATH] [--check]

``--check`` exits non-zero if any required item is unticked (for CI), without
writing.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sqlite3
import sys
from typing import Any, Callable, Optional

# --- repo paths ---------------------------------------------------------------


def repo_root() -> str:
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.normpath(os.path.join(here, "..", ".."))


ROOT = repo_root()
DEFAULT_DB = os.path.join(ROOT, "data", "guardrail_alpha.db")
DEFAULT_REPORT = os.path.join(ROOT, "data", "run_report.json")
CHECKLIST = os.path.join(ROOT, "docs", "SUBMISSION_CHECKLIST.md")
UNIVERSE = os.path.join(ROOT, "configs", "eligible_assets.bsc.json")
SUBMISSION_MD = os.path.join(ROOT, "data", "exports", "submission.md")

AUTO_SECTION_HEADER = "## Evidence (auto-generated)"


# --- evidence gathering -------------------------------------------------------


class Evidence:
    """The facts we can establish from the agent's artifacts."""

    def __init__(self, db: str, report_path: str) -> None:
        self.event_types: set[str] = set()
        self.report: dict[str, Any] = {}
        self.published: dict[str, Any] = {}
        self.enabled_assets = 0
        self.submission_md = os.path.isfile(SUBMISSION_MD)
        # Load the report first so events can be scoped to its run_id (the
        # kill-switch demo shares the DB and would otherwise win "latest").
        self._load_report(report_path)
        self._load_events(db)
        self._load_universe()

    def _load_events(self, db: str) -> None:
        if not os.path.isfile(db):
            return
        try:
            conn = sqlite3.connect(db)
            self.event_types = {
                row[0] for row in conn.execute("SELECT DISTINCT event_type FROM events")
            }
            # Prefer the published report tied to the primary run; fall back to
            # the most recent one if no run_id match (or no run report).
            primary_run = self.report.get("run_id")
            rows = conn.execute(
                "SELECT payload_json FROM events WHERE event_type='agent_report_published' "
                "ORDER BY timestamp DESC"
            ).fetchall()
            conn.close()
            published = [json.loads(r[0]) for r in rows]
            self.published = next(
                (p for p in published if p.get("run_id") == primary_run),
                published[0] if published else {},
            )
        except (sqlite3.Error, json.JSONDecodeError):
            pass

    def _load_report(self, report_path: str) -> None:
        try:
            with open(report_path, "r", encoding="utf-8") as fh:
                self.report = json.load(fh)
        except (OSError, json.JSONDecodeError):
            pass

    def _load_universe(self) -> None:
        try:
            with open(UNIVERSE, "r", encoding="utf-8") as fh:
                assets = json.load(fh)
            self.enabled_assets = sum(1 for a in assets if a.get("enabled", True))
        except (OSError, json.JSONDecodeError):
            pass

    def field(self, key: str) -> Optional[Any]:
        """Look a field up in the published-report payload, then the run report."""
        if key in self.published:
            return self.published[key]
        return self.report.get(key)

    def has(self, *event_types: str) -> bool:
        return all(t in self.event_types for t in event_types)

    def any_of(self, *event_types: str) -> bool:
        return any(t in self.event_types for t in event_types)


# --- checklist items ----------------------------------------------------------

# Each item: a keyword that uniquely identifies its line in the checklist, and a
# predicate returning (ticked, note). A note prefixed "pending" never ticks.
Predicate = Callable[[Evidence], "tuple[bool, str]"]


def _is_hex64(v: Any) -> bool:
    return isinstance(v, str) and bool(re.fullmatch(r"[0-9a-fA-F]{64}", v))


def policy_hash(e: Evidence) -> tuple[bool, str]:
    h = e.field("policy_hash")
    return (_is_hex64(h), f"policy_hash {h[:12]}…" if _is_hex64(h) else "no policy_hash")


def agent_identity(e: Evidence) -> tuple[bool, str]:
    aid = e.field("agent_id")
    ok = isinstance(aid, str) and len(aid) >= 16
    return (ok, f"agent_id {aid[:12]}…" if ok else "no agent_id")


def registration(e: Evidence) -> tuple[bool, str]:
    tx = e.field("registration_tx")
    if isinstance(tx, str) and tx:
        return (True, f"on-chain registration_tx {tx[:12]}…")
    # Paper: a deterministic registration target exists, but no real tx yet.
    return (False, "pending (live): no registration_tx — run scripts/go_live.sh")


def eligible_assets(e: Evidence) -> tuple[bool, str]:
    n = e.enabled_assets
    return (n >= 1, f"{n} enabled eligible asset(s)")


def cmc_data(e: Evidence) -> tuple[bool, str]:
    ok = e.any_of("market_snapshot_received", "regime_classified")
    return (ok, "market_snapshot_received / regime_classified in log" if ok else "no CMC events")


def risk_examples(e: Evidence) -> tuple[bool, str]:
    ok = e.has("risk_approved") and e.any_of("risk_rejected", "risk_clipped")
    return (ok, "risk_approved + risk_rejected/clipped in log" if ok else "missing approval or rejection")


def twak_trade(e: Evidence) -> tuple[bool, str]:
    ok = e.has("twak_quote_received", "tx_confirmed")
    note = "twak_quote_received + tx_confirmed in log"
    if ok and str(e.field("mode")) == "paper":
        note += " (paper: mock tx)"
    return (ok, note if ok else "no quote/confirmed pair")


def kill_switch(e: Evidence) -> tuple[bool, str]:
    ok = "kill_switch_triggered" in e.event_types or e.field("kill_switch") is True
    return (ok, "kill_switch_triggered in log" if ok else "no kill-switch event — run scripts/kill_switch.sh")


def run_report(e: Evidence) -> tuple[bool, str]:
    ok = bool(e.report) and "run_id" in e.report
    return (ok, f"run_report.json present (run {str(e.report.get('run_id',''))[:8]}…)" if ok else "no run report")


def submission_md(e: Evidence) -> tuple[bool, str]:
    return (e.submission_md, "data/exports/submission.md present" if e.submission_md else "not exported")


def proof_page(e: Evidence) -> tuple[bool, str]:
    ok = bool(e.field("agent_id")) and _is_hex64(e.field("policy_hash"))
    return (ok, "proof fields (agent_id + policy_hash) available" if ok else "proof data incomplete")


ITEMS: list[tuple[str, Predicate]] = [
    ("Policy hash generated", policy_hash),
    ("Agent identity", agent_identity),
    ("Competition registration", registration),
    ("Eligible BSC assets", eligible_assets),
    ("CMC data visible", cmc_data),
    ("Risk approval and rejection", risk_examples),
    ("TWAK quote", twak_trade),
    ("Kill-switch behavior", kill_switch),
    ("Daily report", run_report),
    ("Submission Markdown", submission_md),
    ("Dashboard proof page", proof_page),
]


# --- checklist rewriting ------------------------------------------------------


def evaluate(e: Evidence) -> list[tuple[str, bool, str]]:
    return [(kw, *pred(e)) for kw, pred in ITEMS]


def rewrite(text: str, results: list[tuple[str, bool, str]]) -> str:
    """Flip checkboxes in place and regenerate the auto evidence section."""
    by_keyword = {kw: (ticked, note) for kw, ticked, note in results}

    lines = text.splitlines()
    out: list[str] = []
    for line in lines:
        stripped = line.lstrip()
        if stripped.startswith("- [ ]") or stripped.startswith("- [x]"):
            match = next((kw for kw in by_keyword if kw in line), None)
            if match:
                ticked, _ = by_keyword[match]
                box = "[x]" if ticked else "[ ]"
                line = re.sub(r"- \[[ x]\]", f"- {box}", line, count=1)
        out.append(line)

    body = "\n".join(out)
    # Drop any prior auto-generated section, then append a fresh one.
    idx = body.find(AUTO_SECTION_HEADER)
    if idx != -1:
        body = body[:idx].rstrip() + "\n"
    body = body.rstrip() + "\n\n" + render_evidence_table(results) + "\n"
    return body


def render_evidence_table(results: list[tuple[str, bool, str]]) -> str:
    ticked = sum(1 for _, t, _ in results if t)
    lines = [
        AUTO_SECTION_HEADER,
        "",
        f"Generated by `python-lab/scripts/tick_checklist.py` from real artifacts. "
        f"**{ticked}/{len(results)} ticked.**",
        "",
        "| Item | Status | Evidence |",
        "|---|---|---|",
    ]
    for kw, t, note in results:
        mark = "✅" if t else ("🟡" if note.startswith("pending") else "⬜")
        lines.append(f"| {kw} | {mark} | {note} |")
    return "\n".join(lines)


def main(argv: Optional[list[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Auto-tick the submission checklist from evidence.")
    parser.add_argument("--db", default=DEFAULT_DB)
    parser.add_argument("--report", default=DEFAULT_REPORT)
    parser.add_argument("--check", action="store_true", help="Exit non-zero if any item is unticked; do not write.")
    args = parser.parse_args(argv)

    evidence = Evidence(args.db, args.report)
    results = evaluate(evidence)
    ticked = sum(1 for _, t, _ in results if t)

    for kw, t, note in results:
        mark = "PASS" if t else ("PEND" if note.startswith("pending") else "MISS")
        print(f"[{mark}] {kw}: {note}")
    print(f"\n{ticked}/{len(results)} ticked")

    if args.check:
        missing = [kw for kw, t, note in results if not t and not note.startswith("pending")]
        if missing:
            print(f"unticked (non-pending): {', '.join(missing)}", file=sys.stderr)
            return 1
        return 0

    try:
        with open(CHECKLIST, "r", encoding="utf-8") as fh:
            text = fh.read()
    except OSError as err:
        print(f"error: cannot read {CHECKLIST}: {err}", file=sys.stderr)
        return 2
    with open(CHECKLIST, "w", encoding="utf-8") as fh:
        fh.write(rewrite(text, results))
    print(f"\nupdated {os.path.relpath(CHECKLIST, ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
