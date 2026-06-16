// Tiny ANSI + table helpers — no dependencies.

const ANSI = {
  reset: "\x1b[0m",
  bold: "\x1b[1m",
  dim: "\x1b[2m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  red: "\x1b[31m",
  cyan: "\x1b[36m",
} as const;

type Color = keyof typeof ANSI;

export function color(text: string, c: Color): string {
  return `${ANSI[c]}${text}${ANSI.reset}`;
}

export function heading(text: string): string {
  return color(text, "bold");
}

/** Render a list of `[label, value]` rows as an aligned two-column table. */
export function kvTable(rows: Array<[string, string]>): string {
  const width = rows.reduce((m, [k]) => Math.max(m, k.length), 0);
  return rows.map(([k, v]) => `  ${color(k.padEnd(width), "dim")}  ${v}`).join("\n");
}

/** Render rows under headers as an aligned table. */
export function table(headers: string[], rows: string[][]): string {
  const widths = headers.map((h, i) =>
    Math.max(h.length, ...rows.map((r) => (r[i] ?? "").length)),
  );
  const fmt = (cells: string[]): string =>
    cells.map((c, i) => (c ?? "").padEnd(widths[i] ?? 0)).join("  ");
  const head = color(fmt(headers), "bold");
  const body = rows.map(fmt).join("\n");
  return `${head}\n${body}`;
}

export function ok(text: string): string {
  return color(text, "green");
}
export function warn(text: string): string {
  return color(text, "yellow");
}
export function bad(text: string): string {
  return color(text, "red");
}
