"""Guardrail reporting package.

Reads the agent's SQLite event log and run report, computes summary risk
metrics, and renders a self-contained HTML report (or a text summary).

Pure standard library: no third-party dependencies.
"""

from .data import EventLog, RunReport, load_event_log, load_run_report
from .metrics import ReportMetrics, compute_metrics
from .html import render_html
from .text import render_text

__all__ = [
    "EventLog",
    "RunReport",
    "load_event_log",
    "load_run_report",
    "ReportMetrics",
    "compute_metrics",
    "render_html",
    "render_text",
]

__version__ = "0.1.0"
