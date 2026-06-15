"""Self-contained HTML renderer for the Guardrail Alpha decision journal.

This module is a thin *presentation* layer on top of
:mod:`guardrail_lab.journal`. It NEVER recomputes any analytic: it builds the
journal from the event log (reusing :func:`journal.build_journal_from_db`),
asks :func:`journal.render_markdown` for the Markdown narrative, and converts
that Markdown into a single, fully self-contained HTML document.

The Markdown->HTML conversion and the dark-theme stylesheet/document wrapper
are *imported* from :mod:`guardrail_lab.dossier_html` rather than
re-implemented, so the journal and the dossier share one renderer and one look.

"Self-contained" means one ``<html>`` document with an inline ``<style>``
block, a dark theme, and NO external CSS/JS/CDN references.

Like :mod:`guardrail_lab.journal`, this never raises on missing data: an empty
event log yields the journal's "no data" Markdown skeleton, which is converted
to a valid HTML skeleton.

Standard-library only.
"""

from __future__ import annotations

from pathlib import Path

from . import journal as jn
from .dossier_html import markdown_to_html_body
from .dossier_html import _wrap_document as wrap_document

#: Default location of the event-log database.
DEFAULT_DB = "data/guardrail_alpha.db"
#: Default page title for the rendered HTML journal.
DEFAULT_TITLE = "Guardrail Alpha — Decision Journal"
#: Default maximum scored assets to list per cycle (matches the CLI default).
DEFAULT_TOP_N = 5


def build_journal_html(
    db_path: str = DEFAULT_DB,
    top_n: int = DEFAULT_TOP_N,
    title: str = DEFAULT_TITLE,
) -> str:
    """Build a single self-contained HTML decision journal.

    Loads the event log and builds the journal via
    :func:`guardrail_lab.journal.build_journal_from_db`, renders it to Markdown
    with :func:`guardrail_lab.journal.render_markdown`, and converts that to one
    self-contained HTML document (inline dark-theme CSS, no external
    CSS/JS/CDN references) using the shared converter from
    :mod:`guardrail_lab.dossier_html`.

    Never raises on missing data: an empty event log produces the journal's
    "no data" skeleton, which is wrapped into a valid HTML skeleton.

    Args:
        db_path: Path to the SQLite event-log database.
        top_n: Maximum scored assets to list per cycle (non-positive falls back
            to :data:`DEFAULT_TOP_N`).
        title: Page ``<title>`` and document heading.

    Returns:
        The journal rendered as a complete HTML document string.
    """
    effective_top_n = top_n if top_n and top_n > 0 else DEFAULT_TOP_N
    journal = jn.build_journal_from_db(db_path)
    markdown = jn.render_markdown(journal, top_n=effective_top_n)
    body = markdown_to_html_body(markdown)
    return wrap_document(title, body)


def write_journal_html(
    path: str,
    db_path: str = DEFAULT_DB,
    top_n: int = DEFAULT_TOP_N,
    title: str = DEFAULT_TITLE,
) -> str:
    """Build the HTML journal and write it to ``path``; return the HTML.

    Creates the parent directory tree if needed (``mkdir -p`` semantics).

    Args:
        path: Destination file path for the HTML journal.
        db_path: Path to the SQLite event-log database.
        top_n: Maximum scored assets to list per cycle.
        title: Page ``<title>`` and document heading.

    Returns:
        The journal HTML string that was written.
    """
    document = build_journal_html(db_path=db_path, top_n=top_n, title=title)
    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(document, encoding="utf-8")
    return document
