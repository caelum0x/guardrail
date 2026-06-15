"""Self-contained HTML report *bundle* generator for Guardrail Alpha.

This module is a thin *composition* layer that stitches together the existing
self-contained HTML renderers into one browsable folder of reports. It NEVER
recomputes any analytic and NEVER re-implements a Markdown converter — it only
calls the established builders and the shared dossier converter:

* :func:`guardrail_lab.dossier_html.build_dossier_html` for ``dossier.html``,
* :func:`guardrail_lab.journal_html.build_journal_html` for ``journal.html``,
* :func:`guardrail_lab.ensemble_compare.compare_all` +
  :func:`guardrail_lab.ensemble_compare.render_markdown_all` to produce the
  ensemble Markdown, converted to HTML via the *shared* dossier converter
  (:func:`guardrail_lab.dossier_html.markdown_to_html_body` and the document
  wrapper) so the ensemble page shares the dark theme and look, and
* a small ``index.html`` landing page (built with the same wrapper) that links
  to each report with a short description.

"Self-contained" matches the rest of the lab: every page is one ``<html>``
document with an inline dark-theme ``<style>`` block and NO external
CSS/JS/CDN references.

Like the renderers it wraps, this never raises on missing data: each builder
degrades to its own valid "no data" skeleton, so the bundle is always a
complete, well-formed set of pages even on a fresh checkout.

Standard-library only.
"""

from __future__ import annotations

import html
from pathlib import Path

from .dossier_html import build_dossier_html, markdown_to_html_body
from .dossier_html import _wrap_document as wrap_document
from .ensemble import (
    DEFAULT_CONFIG_PATH as DEFAULT_ENSEMBLE_CONFIG,
    DEFAULT_SKILLS_ROOT,
)
from .ensemble_compare import compare_all, render_markdown_all
from .journal_html import build_journal_html

#: Default output directory for the rendered bundle.
DEFAULT_OUT_DIR = "data/reports"
#: Default location of the event-log database.
DEFAULT_DB = "data/guardrail_alpha.db"
#: Default location of the agent's JSON run report.
DEFAULT_REPORT = "data/run_report.json"

#: Page titles for each generated report.
INDEX_TITLE = "Guardrail Alpha — Report Bundle"
DOSSIER_TITLE = "Guardrail Alpha — Research Dossier"
JOURNAL_TITLE = "Guardrail Alpha — Decision Journal"
ENSEMBLE_TITLE = "Guardrail Alpha — Ensemble vs. Single Skills"

#: File names written into the bundle directory.
INDEX_FILE = "index.html"
DOSSIER_FILE = "dossier.html"
JOURNAL_FILE = "journal.html"
ENSEMBLE_FILE = "ensemble.html"

#: The report cards rendered on the index page: (file, title, description).
_REPORT_CARDS: tuple[tuple[str, str, str], ...] = (
    (
        DOSSIER_FILE,
        "Research Dossier",
        "One synthesized research dossier: run summary, headline performance "
        "metrics, per-asset attribution, regime transition / time-in-regime / "
        "exposure analytics, and drawdown (incl. worst episodes).",
    ),
    (
        JOURNAL_FILE,
        "Decision Journal",
        "A human-readable, per-cycle decision journal reconstructed from the "
        "event log: what was classified, scored, proposed, and why — cycle by "
        "cycle.",
    ),
    (
        ENSEMBLE_FILE,
        "Ensemble vs. Single Skills",
        "Across every regime, the blended ensemble book compared against each "
        "single skill: concentration (HHI / effective positions), overlap, and "
        "diversification deltas. Advisory only — the Rust risk engine remains "
        "the sole execution gate.",
    ),
)

#: Short blurb shown beneath the index heading.
_INDEX_INTRO = (
    "Self-contained HTML reports for the Guardrail Alpha run. Every page is "
    "standalone (inline dark-theme CSS, no external assets) and degrades to a "
    "clear no-data skeleton when the agent has not produced data yet."
)


def _build_ensemble_html(
    config_path: str = DEFAULT_ENSEMBLE_CONFIG,
    skills_root: str = DEFAULT_SKILLS_ROOT,
    title: str = ENSEMBLE_TITLE,
) -> str:
    """Build the self-contained ensemble-comparison HTML page.

    Reuses :func:`compare_all` + :func:`render_markdown_all` (recomputing
    nothing) to obtain the ``--all`` Markdown, then converts it with the shared
    dossier converter so the page shares the dark theme. Never raises: an empty
    comparison list yields :func:`render_markdown_all`'s valid no-data note.

    Args:
        config_path: Path to the ensemble blend-weights config.
        skills_root: Root directory holding the skill directories.
        title: Page ``<title>`` and document heading.

    Returns:
        The ensemble comparison rendered as a complete HTML document string.
    """
    comparisons = compare_all(config_path=config_path, skills_root=skills_root)
    markdown = render_markdown_all(comparisons)
    body = markdown_to_html_body(markdown)
    return wrap_document(title, body)


def _render_index_links() -> str:
    """Render the report list as an HTML ``<ul>`` of real relative-link cards.

    Each card links to a report file by relative name (so the bundle is fully
    portable, with no external references) and carries a short description. The
    Markdown converter shared with the dossier has no link rule, so the anchors
    are built here directly; every value is HTML-escaped. The surrounding theme
    still comes from the shared document wrapper.

    Returns:
        An HTML ``<ul>`` fragment for the index report list.
    """
    items: list[str] = ["<ul>"]
    for file_name, card_title, description in _REPORT_CARDS:
        href = html.escape(file_name, quote=True)
        link_text = html.escape(card_title, quote=False)
        desc = html.escape(description, quote=False)
        items.append(
            f'<li><a href="{href}"><strong>{link_text}</strong></a> '
            f"— {desc}</li>"
        )
    items.append("</ul>")
    return "\n".join(items)


def _build_index_html(title: str = INDEX_TITLE) -> str:
    """Build the index landing page as a self-contained HTML document.

    The heading and intro are rendered through the shared dossier converter so
    they inherit the same dark theme, and the report list is appended as real
    HTML anchors (the shared converter has no Markdown-link rule). Links are
    relative file names, so the bundle is fully portable and contains no
    external references.

    Args:
        title: Page ``<title>`` and document heading.

    Returns:
        The index page as a complete HTML document string.
    """
    prose = markdown_to_html_body(
        "\n".join([f"# {title}", "", _INDEX_INTRO, "", "## Reports"])
    )
    body = f"{prose}\n{_render_index_links()}"
    return wrap_document(title, body)


def build_bundle(
    out_dir: str = DEFAULT_OUT_DIR,
    db_path: str = DEFAULT_DB,
    report_path: str = DEFAULT_REPORT,
    config_path: str = DEFAULT_ENSEMBLE_CONFIG,
    skills_root: str = DEFAULT_SKILLS_ROOT,
) -> list[str]:
    """Generate a folder of self-contained HTML reports and return their paths.

    Creates ``out_dir`` (``mkdir -p`` semantics) and writes four pages:

    * ``dossier.html`` — via :func:`dossier_html.build_dossier_html`,
    * ``journal.html`` — via :func:`journal_html.build_journal_html`,
    * ``ensemble.html`` — the ``ensemble-compare --all`` Markdown rendered
      through the shared dossier converter, and
    * ``index.html`` — a landing page linking to the three reports.

    Reuses the existing renderers and converter; nothing is recomputed or
    re-implemented here. Never raises on missing data: each underlying builder
    produces a valid no-data skeleton, so the bundle is always complete and
    well-formed.

    Args:
        out_dir: Destination directory for the bundle.
        db_path: Path to the SQLite event-log database.
        report_path: Path to the agent's JSON run report.
        config_path: Path to the ensemble blend-weights config.
        skills_root: Root directory holding the skill directories.

    Returns:
        The list of written file paths, with ``index.html`` first.
    """
    destination = Path(out_dir)
    destination.mkdir(parents=True, exist_ok=True)

    pages: dict[str, str] = {
        DOSSIER_FILE: build_dossier_html(
            db_path=db_path, report_path=report_path, title=DOSSIER_TITLE
        ),
        JOURNAL_FILE: build_journal_html(db_path=db_path, title=JOURNAL_TITLE),
        ENSEMBLE_FILE: _build_ensemble_html(
            config_path=config_path, skills_root=skills_root
        ),
        INDEX_FILE: _build_index_html(),
    }

    written: list[str] = []
    # index.html first in the returned list (it is the entry point).
    for file_name in (INDEX_FILE, DOSSIER_FILE, JOURNAL_FILE, ENSEMBLE_FILE):
        path = destination / file_name
        path.write_text(pages[file_name], encoding="utf-8")
        written.append(str(path))

    return written
