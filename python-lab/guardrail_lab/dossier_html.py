"""Self-contained HTML renderer for the Guardrail Alpha research dossier.

This module is a thin *presentation* layer on top of
:func:`guardrail_lab.dossier.build_dossier`. It NEVER recomputes any analytic:
it asks :func:`build_dossier` for the Markdown dossier and then converts that
Markdown into a single, fully self-contained HTML document.

"Self-contained" means:

* one ``<html>`` document with an inline ``<style>`` block,
* a dark theme,
* NO external CSS/JS/CDN references (no ``<link href="http...">`` and no
  ``<script src="http...">``).

The Markdown->HTML conversion here is deliberately small and dependency-free.
It covers exactly the constructs that :func:`build_dossier` emits:

* ATX headings (``#``, ``##``, ``###``),
* fenced code blocks (```` ``` ````),
* GitHub-style pipe tables (with a ``---`` separator row, optional alignment),
* bullet lists (``- ``),
* horizontal rules (a line that is only dashes/underscores/asterisks),
* paragraphs, and within text spans: ``**bold**`` and inline ``` `code` ```.

All text is HTML-escaped before it is emitted, so arbitrary data values in the
dossier can never break the document or inject markup.

Standard-library only.
"""

from __future__ import annotations

import html
import re
from pathlib import Path

from .dossier import DEFAULT_DB, DEFAULT_REPORT, build_dossier

#: Default page title for the rendered HTML dossier.
DEFAULT_TITLE = "Guardrail Alpha — Research Dossier"

#: Inline dark-theme stylesheet. Kept self-contained (no @import / url()).
_STYLE = """
:root {
  --bg: #0d1117;
  --panel: #161b22;
  --border: #30363d;
  --text: #e6edf3;
  --muted: #8b949e;
  --accent: #58a6ff;
  --code-bg: #1f2630;
  --code-text: #ffa657;
  --row-alt: #11161d;
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body {
  margin: 0;
  padding: 2.5rem 1rem 4rem;
  background: var(--bg);
  color: var(--text);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    Helvetica, Arial, sans-serif;
  line-height: 1.6;
  font-size: 16px;
}
main {
  max-width: 900px;
  margin: 0 auto;
}
h1, h2, h3 {
  line-height: 1.25;
  font-weight: 600;
  color: var(--text);
}
h1 {
  font-size: 2rem;
  margin: 0 0 1.25rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--border);
}
h2 {
  font-size: 1.4rem;
  margin: 2.25rem 0 1rem;
  padding-bottom: 0.35rem;
  border-bottom: 1px solid var(--border);
}
h3 {
  font-size: 1.1rem;
  margin: 1.75rem 0 0.75rem;
  color: var(--accent);
}
p { margin: 0.75rem 0; }
a { color: var(--accent); }
em { color: var(--muted); }
strong { color: var(--text); font-weight: 600; }
hr {
  border: none;
  border-top: 1px solid var(--border);
  margin: 2rem 0;
}
ul {
  margin: 0.75rem 0;
  padding-left: 1.4rem;
}
li { margin: 0.3rem 0; }
code {
  background: var(--code-bg);
  color: var(--code-text);
  padding: 0.15em 0.4em;
  border-radius: 4px;
  font-family: "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
  font-size: 0.9em;
}
pre {
  background: var(--code-bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 1rem;
  overflow-x: auto;
  margin: 1rem 0;
}
pre code {
  background: none;
  color: var(--text);
  padding: 0;
  border-radius: 0;
  font-size: 0.85rem;
}
table {
  border-collapse: collapse;
  width: 100%;
  margin: 1rem 0;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  font-size: 0.92rem;
}
th, td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--border);
  text-align: left;
}
th {
  background: #1c232c;
  font-weight: 600;
  color: var(--text);
}
tbody tr:nth-child(even) { background: var(--row-alt); }
tbody tr:last-child td { border-bottom: none; }
.text-right { text-align: right; }
.text-center { text-align: center; }
"""


def _render_inline(text: str) -> str:
    """Render inline Markdown (``**bold**`` and ``` `code` ```) to HTML.

    The input is fully HTML-escaped first, then the (escaped) bold and inline
    code markers are translated into the corresponding tags. Inline code is
    handled first so that ``**`` inside backticks is treated as literal text.

    Args:
        text: A raw Markdown text span (no block-level constructs).

    Returns:
        An HTML-safe string with ``<strong>``/``<code>`` spans applied.
    """
    escaped = html.escape(text, quote=False)

    # Inline code first: capture content between single backticks and wrap it,
    # escaping any leftover ``**`` so it is not later treated as bold.
    def _code_sub(match: re.Match[str]) -> str:
        inner = match.group(1).replace("**", "&#42;&#42;")
        return f"<code>{inner}</code>"

    escaped = re.sub(r"`([^`]+)`", _code_sub, escaped)

    # Bold: **...** -> <strong>...</strong> (non-greedy, no nested ``**``).
    escaped = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", escaped)

    # Italic: _..._ -> <em>...</em>. Require the markers to hug the content and
    # sit on word boundaries so underscores inside identifiers (e.g.
    # ``risk_on``) are left untouched.
    escaped = re.sub(
        r"(?<![\w])_(?=\S)(.+?)(?<=\S)_(?![\w])",
        r"<em>\1</em>",
        escaped,
    )

    return escaped


def _is_hr(line: str) -> bool:
    """Return ``True`` if ``line`` is a Markdown horizontal rule."""
    stripped = line.strip()
    if len(stripped) < 3:
        return False
    return set(stripped) in ({"-"}, {"*"}, {"_"})


def _split_table_row(line: str) -> list[str]:
    """Split a pipe-table row into trimmed cell strings.

    Leading/trailing pipes are tolerated and removed so both ``| a | b |`` and
    ``a | b`` parse to ``["a", "b"]``.
    """
    trimmed = line.strip()
    if trimmed.startswith("|"):
        trimmed = trimmed[1:]
    if trimmed.endswith("|"):
        trimmed = trimmed[:-1]
    return [cell.strip() for cell in trimmed.split("|")]


def _is_table_separator(line: str) -> bool:
    """Return ``True`` if ``line`` is a pipe-table header separator row.

    A separator row's cells consist only of dashes and optional alignment
    colons, e.g. ``| --- | ---: | :---: |``.
    """
    if "|" not in line:
        return False
    cells = _split_table_row(line)
    if not cells:
        return False
    for cell in cells:
        if not cell:
            return False
        if not re.fullmatch(r":?-{1,}:?", cell):
            return False
    return True


def _cell_alignment(separator_cell: str) -> str:
    """Map a separator cell (``---``/``:--``/``--:``/``:-:``) to a CSS class."""
    left = separator_cell.startswith(":")
    right = separator_cell.endswith(":")
    if left and right:
        return "text-center"
    if right:
        return "text-right"
    return ""


def _render_table(
    header: list[str], separator: list[str], rows: list[list[str]]
) -> list[str]:
    """Render a parsed pipe table to HTML lines.

    Args:
        header: Header cell texts.
        separator: Separator cells (used only for column alignment).
        rows: Body rows, each a list of cell texts.

    Returns:
        HTML lines for one ``<table>`` element.
    """
    alignments = [_cell_alignment(cell) for cell in separator]

    def _class_attr(index: int) -> str:
        align = alignments[index] if index < len(alignments) else ""
        return f' class="{align}"' if align else ""

    out: list[str] = ["<table>", "<thead>", "<tr>"]
    for index, cell in enumerate(header):
        out.append(f"<th{_class_attr(index)}>{_render_inline(cell)}</th>")
    out.extend(["</tr>", "</thead>", "<tbody>"])

    for row in rows:
        out.append("<tr>")
        for index, cell in enumerate(row):
            out.append(f"<td{_class_attr(index)}>{_render_inline(cell)}</td>")
        out.append("</tr>")

    out.extend(["</tbody>", "</table>"])
    return out


def markdown_to_html_body(markdown: str) -> str:
    """Convert dossier Markdown into an HTML fragment (no ``<html>`` wrapper).

    Supports the subset of Markdown emitted by
    :func:`guardrail_lab.dossier.build_dossier`: ATX headings, fenced code
    blocks, GitHub pipe tables, bullet lists, horizontal rules, paragraphs,
    and inline bold / code spans. All text is HTML-escaped.

    Args:
        markdown: The dossier Markdown source.

    Returns:
        An HTML body fragment as a single string.
    """
    lines = markdown.splitlines()
    out: list[str] = []
    index = 0
    total = len(lines)

    while index < total:
        line = lines[index]
        stripped = line.strip()

        # Blank line -> skip (paragraph / block boundaries handled per-block).
        if not stripped:
            index += 1
            continue

        # Fenced code block.
        if stripped.startswith("```"):
            code_lines: list[str] = []
            index += 1
            while index < total and not lines[index].strip().startswith("```"):
                code_lines.append(lines[index])
                index += 1
            index += 1  # consume the closing fence (if present)
            body = html.escape("\n".join(code_lines), quote=False)
            out.append(f"<pre><code>{body}</code></pre>")
            continue

        # Horizontal rule.
        if _is_hr(line):
            out.append("<hr />")
            index += 1
            continue

        # Headings (#, ##, ###; deeper levels clamp to h3).
        heading = re.match(r"^(#{1,6})\s+(.*)$", stripped)
        if heading:
            level = min(len(heading.group(1)), 3)
            content = _render_inline(heading.group(2).strip())
            out.append(f"<h{level}>{content}</h{level}>")
            index += 1
            continue

        # Pipe table: current line has a pipe and the next line is a separator.
        if "|" in line and index + 1 < total and _is_table_separator(
            lines[index + 1]
        ):
            header = _split_table_row(line)
            separator = _split_table_row(lines[index + 1])
            index += 2
            rows: list[list[str]] = []
            while index < total and "|" in lines[index] and lines[
                index
            ].strip():
                rows.append(_split_table_row(lines[index]))
                index += 1
            out.extend(_render_table(header, separator, rows))
            continue

        # Bullet list.
        if re.match(r"^[-*]\s+", stripped):
            items: list[str] = []
            while index < total and re.match(
                r"^[-*]\s+", lines[index].strip()
            ):
                item_text = re.sub(
                    r"^[-*]\s+", "", lines[index].strip(), count=1
                )
                items.append(f"<li>{_render_inline(item_text)}</li>")
                index += 1
            out.append("<ul>")
            out.extend(items)
            out.append("</ul>")
            continue

        # Paragraph: gather consecutive non-blank, non-block lines.
        para_lines: list[str] = []
        while index < total:
            current = lines[index]
            current_stripped = current.strip()
            if not current_stripped:
                break
            if current_stripped.startswith("```"):
                break
            if _is_hr(current):
                break
            if re.match(r"^#{1,6}\s+", current_stripped):
                break
            if re.match(r"^[-*]\s+", current_stripped):
                break
            if "|" in current and index + 1 < total and _is_table_separator(
                lines[index + 1]
            ):
                break
            para_lines.append(current_stripped)
            index += 1
        rendered = "<br />".join(_render_inline(p) for p in para_lines)
        out.append(f"<p>{rendered}</p>")

    return "\n".join(out)


def _wrap_document(title: str, body: str) -> str:
    """Wrap an HTML body fragment in a complete self-contained document.

    Args:
        title: Page title (HTML-escaped before insertion).
        body: HTML body fragment (already escaped / rendered).

    Returns:
        A complete ``<!DOCTYPE html>`` document string.
    """
    safe_title = html.escape(title, quote=True)
    return (
        "<!DOCTYPE html>\n"
        '<html lang="en">\n'
        "<head>\n"
        '<meta charset="utf-8" />\n'
        '<meta name="viewport" '
        'content="width=device-width, initial-scale=1" />\n'
        f"<title>{safe_title}</title>\n"
        f"<style>{_STYLE}</style>\n"
        "</head>\n"
        "<body>\n"
        "<main>\n"
        f"{body}\n"
        "</main>\n"
        "</body>\n"
        "</html>\n"
    )


def build_dossier_html(
    db_path: str = DEFAULT_DB,
    report_path: str = DEFAULT_REPORT,
    title: str = DEFAULT_TITLE,
) -> str:
    """Build a single self-contained HTML research dossier.

    Calls :func:`guardrail_lab.dossier.build_dossier` to obtain the Markdown
    dossier (reusing all existing analytics, recomputing nothing) and converts
    it into one self-contained HTML document with an inline dark-theme
    stylesheet and no external CSS/JS/CDN references.

    Like :func:`build_dossier`, this never raises on missing data: the
    no-data Markdown skeleton is converted to a valid HTML skeleton.

    Args:
        db_path: Path to the SQLite event-log database.
        report_path: Path to the agent's JSON run report.
        title: Page ``<title>`` and document heading override.

    Returns:
        The dossier rendered as a complete HTML document string.
    """
    markdown = build_dossier(db_path=db_path, report_path=report_path)
    body = markdown_to_html_body(markdown)
    return _wrap_document(title, body)


def write_dossier_html(
    path: str,
    db_path: str = DEFAULT_DB,
    report_path: str = DEFAULT_REPORT,
    title: str = DEFAULT_TITLE,
) -> str:
    """Build the HTML dossier and write it to ``path``; return the HTML.

    Creates the parent directory tree if needed (``mkdir -p`` semantics).

    Args:
        path: Destination file path for the HTML dossier.
        db_path: Path to the SQLite event-log database.
        report_path: Path to the agent's JSON run report.
        title: Page ``<title>`` and document heading override.

    Returns:
        The dossier HTML string that was written.
    """
    document = build_dossier_html(
        db_path=db_path, report_path=report_path, title=title
    )
    destination = Path(path)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(document, encoding="utf-8")
    return document
