// Pure rendering helpers for the Guardrail operator CLI.
//
// These mirror clients/go/cmd/guardrailctl/{render.go,*.go}: each function takes
// a typed response and returns the human-readable text block (it does not print,
// so it stays pure and testable). All formatting is allocation-light and never
// throws on missing/nullable fields.

import type {
  CompeteResponse,
  EnsembleResponse,
  JournalCycle,
  JournalResponse,
  ProofVerifyResponse,
  RegimeResponse,
  ServerCheck,
  SkillDetail,
  SkillsResponse,
  SnapshotsResponse,
} from "./index.js";

// --- Scalar helpers -------------------------------------------------------

/** Render an empty/whitespace string as a dash so columns never collapse. */
export function orDash(s: string | null | undefined): string {
  return s != null && s.trim() !== "" ? s : "-";
}

/** Render a nullable number as text, or "-" when null/undefined. */
export function numOrDash(n: number | null | undefined): string {
  return n == null ? "-" : String(n);
}

/** Render a weight/score compactly (JS already trims trailing zeros). */
export function formatFloat(f: number | null | undefined): string {
  return f == null ? "-" : String(f);
}

/** Render a nullable epoch-millis timestamp as a UTC ISO-8601 string, or "-". */
export function formatMillis(ms: number | null | undefined): string {
  return ms == null ? "-" : new Date(ms).toISOString();
}

/** Render a string list as a comma-separated value, or "-" when empty. */
export function joinOrDash(values: string[] | null | undefined): string {
  return values && values.length > 0 ? values.join(", ") : "-";
}

// --- Table layout ---------------------------------------------------------

/**
 * Render rows as a left-aligned, space-padded table (tabwriter equivalent). The
 * first row is treated as the header. Columns are padded to the widest cell.
 */
export function table(rows: string[][], indent = ""): string {
  if (rows.length === 0) return "";
  const cols = Math.max(...rows.map((r) => r.length));
  const widths = new Array<number>(cols).fill(0);
  for (const row of rows) {
    for (let i = 0; i < cols; i++) {
      widths[i] = Math.max(widths[i], (row[i] ?? "").length);
    }
  }
  return rows
    .map((row) => {
      const cells = row.map((cell, i) =>
        i === cols - 1 ? cell : (cell ?? "").padEnd(widths[i] + 2),
      );
      return indent + cells.join("").trimEnd();
    })
    .join("\n");
}

// --- status (watch single tick) ------------------------------------------

/** Build the one-line status string from a regime + compete read. */
export function renderStatusLine(
  stamp: string,
  regime: RegimeResponse | null,
  compete: CompeteResponse | null,
): string {
  const left = regime
    ? `regime=${orDash(regime.regime)} exposure=${orDash(regime.exposure_multiplier)} f/g=${regime.inputs.fear_greed}`
    : "regime=offline";
  const right = compete
    ? `registered=${compete.registered} trades=${compete.confirmed_trades} daily=${compete.daily_trade_satisfied} kill=${compete.kill_switch}`
    : "compete=offline";
  return `[${stamp}] ${left}  |  ${right}`;
}

// --- ensemble -------------------------------------------------------------

export function renderEnsemble(resp: EnsembleResponse): string {
  const lines: string[] = [
    `${resp.name} v${resp.version}  reserve=${resp.reserve_symbol}  max_risk=${formatFloat(resp.max_risk_allocation_pct)}%`,
    `current regime: ${orDash(resp.current_regime)}`,
    "",
  ];

  if (resp.skills.length === 0 || resp.regimes.length === 0) {
    lines.push("(no ensemble weights reported)");
    return lines.join("\n");
  }

  const current = orDash(resp.current_regime);
  const header = ["REGIME", ...resp.skills.map((s) => s.id)];
  const rows = [header];
  for (const row of resp.regimes) {
    const label = row.regime === current ? `* ${row.regime}` : row.regime;
    rows.push([label, ...resp.skills.map((s) => formatFloat(row.weights[s.id]))]);
  }
  lines.push(table(rows));

  lines.push("", "skills:");
  for (const skill of resp.skills) {
    lines.push(`  ${skill.id.padEnd(28)} ${skill.label}`);
  }
  if (resp.active_weights && Object.keys(resp.active_weights).length > 0) {
    lines.push("(* marks the currently active regime row)");
  }
  return lines.join("\n");
}

// --- journal --------------------------------------------------------------

export function renderJournal(resp: JournalResponse): string {
  const lines: string[] = [
    `decision journal: ${resp.total_cycles} cycles, ${resp.total_events} events, ${resp.confirmed_trades_total} confirmed trades`,
  ];
  if (resp.run_ids.length > 0) {
    lines.push(`runs: ${resp.run_ids.join(", ")}`);
  }
  if (resp.cycles.length === 0) {
    lines.push("", "(no cycles recorded — empty or unavailable event log)");
    return lines.join("\n");
  }
  for (const cycle of resp.cycles) {
    lines.push("", renderCycle(cycle));
  }
  return lines.join("\n");
}

function renderCycle(cycle: JournalCycle): string {
  const lines: string[] = [
    `#${cycle.index}  regime=${orDash(cycle.regime)}  ${orDash(cycle.started_at)} -> ${orDash(cycle.ended_at)}`,
  ];
  if (cycle.run_id !== "") lines.push(`    run: ${cycle.run_id}`);
  if (cycle.headline.trim() !== "") lines.push(`    headline: ${cycle.headline}`);

  if (cycle.top_assets.length > 0) {
    const maxShown = 5;
    const parts: string[] = [];
    for (let i = 0; i < cycle.top_assets.length; i++) {
      if (i >= maxShown) {
        parts.push(`(+${cycle.top_assets.length - maxShown} more)`);
        break;
      }
      const a = cycle.top_assets[i];
      parts.push(`${a.symbol}(${formatFloat(a.score)})`);
    }
    lines.push(`    top assets: ${parts.join(", ")}`);
  }
  if (cycle.orders.length > 0) {
    const parts = cycle.orders.map((o) => `${o.from}->${o.to} $${formatFloat(o.amount_usd)}`);
    lines.push(`    orders: ${parts.join(", ")}`);
  }

  let risk = `    risk: approved=${cycle.risk.approved} clipped=${cycle.risk.clipped} rejected=${cycle.risk.rejected}`;
  if (cycle.risk.rejection_reasons.length > 0) {
    risk += ` (${cycle.risk.rejection_reasons.join("; ")})`;
  }
  lines.push(risk);
  lines.push(
    `    confirmed=${cycle.confirmed_trades}  ending_nav=${orDash(cycle.ending_nav)}  positions=${numOrDash(cycle.positions)}`,
  );
  return lines.join("\n");
}

// --- snapshots ------------------------------------------------------------

export function renderSnapshots(resp: SnapshotsResponse): string {
  const lines: string[] = [
    `snapshot directory: ${orDash(resp.directory)}`,
    `runs discovered: ${resp.runs.length}`,
  ];

  if (resp.runs.length > 0) {
    lines.push("");
    const rows = [["RUN", "MODIFIED"]];
    for (const r of resp.runs) rows.push([r.run_id, formatMillis(r.modified_ms)]);
    lines.push(table(rows));
  }

  const s = resp.latest;
  if (s == null) {
    lines.push("", "(no run summary available — empty or unavailable snapshot directory)");
    return lines.join("\n");
  }

  lines.push("");
  lines.push(`latest run: ${s.run_id}`);
  lines.push(`  cycles=${s.cycle_count} skipped=${s.skipped_lines}`);
  lines.push(`  first=${formatMillis(s.first_timestamp_ms)}  last=${formatMillis(s.last_timestamp_ms)}`);

  if (s.latest_prices.length === 0) {
    lines.push("  latest prices: (none)");
    return lines.join("\n");
  }
  lines.push("  latest prices:");
  const rows = [["SYMBOL", "PRICE_USD"]];
  for (const p of s.latest_prices) rows.push([p.symbol, p.price_usd]);
  lines.push(table(rows, "  "));
  return lines.join("\n");
}

// --- skills ---------------------------------------------------------------

export function renderSkillCatalog(resp: SkillsResponse): string {
  const lines: string[] = [`skill catalog: ${resp.count} skill(s)  (${orDash(resp.index_path)})`];
  if (resp.skills.length === 0) {
    lines.push("", "(no skills published — empty or unavailable index)");
    return lines.join("\n");
  }
  lines.push("");
  const rows = [["ID", "NAME", "REGIMES"]];
  for (const s of resp.skills) rows.push([s.id, orDash(s.name), joinOrDash(s.regimes)]);
  lines.push(table(rows));
  return lines.join("\n");
}

export function renderSkillDetail(id: string, resp: SkillDetail): string {
  if (resp.error != null && resp.error !== "") {
    return `skill "${id}": ${resp.error}`;
  }
  const lines: string[] = [`${orDash(resp.name)}  (${orDash(resp.id)})`];
  if (resp.summary && resp.summary.trim() !== "") lines.push(`  summary: ${resp.summary}`);
  if (resp.description && resp.description.trim() !== "") lines.push(`  description: ${resp.description}`);
  lines.push(`  regimes: ${joinOrDash(resp.regimes)}`);
  if (resp.inputs && resp.inputs.length > 0) lines.push(`  inputs: ${resp.inputs.join(", ")}`);
  lines.push(
    `  eligible universe: ${resp.eligible_universe_size ?? 0}  examples: ${resp.examples_count ?? 0} (on disk: ${resp.examples_on_disk ?? 0})`,
  );
  if (resp.spec_file && resp.spec_file.trim() !== "") lines.push(`  spec file: ${resp.spec_file}`);
  if (resp.spec_sections && resp.spec_sections.length > 0) {
    lines.push(`  spec sections: ${resp.spec_sections.join(", ")}`);
  }
  return lines.join("\n");
}

// --- verify (server-side proof) ------------------------------------------

export function renderVerify(resp: ProofVerifyResponse): string {
  const passed = resp.checks.filter((c) => c.status === "pass").length;
  const failed = resp.checks.length - passed;
  const overall = resp.passed ? "PASS" : "FAIL";
  const lines: string[] = [`proof verification: ${overall}  (${passed} passed, ${failed} failed)`];
  if (resp.report_path) lines.push(`report: ${resp.report_path}`);
  if (resp.reason) lines.push(`reason: ${resp.reason}`);

  if (resp.checks.length === 0) {
    lines.push("", "(no checks reported)");
    return lines.join("\n");
  }

  lines.push("");
  const rows = [["STATUS", "CHECK", "DETAIL"]];
  for (const c of resp.checks) rows.push([verifyStatusLabel(c), c.name, c.detail]);
  lines.push(table(rows));

  if (resp.recomputed_policy_hashes && resp.recomputed_policy_hashes.length > 0) {
    lines.push("", "recomputed policy hashes:");
    for (const h of resp.recomputed_policy_hashes) lines.push(`  ${h.sha256}  ${h.file}`);
  }
  return lines.join("\n");
}

function verifyStatusLabel(c: ServerCheck): string {
  return c.status === "pass" ? "PASS" : "FAIL";
}
