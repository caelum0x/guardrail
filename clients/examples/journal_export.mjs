#!/usr/bin/env node
// Decision journal exporter -- fetches /events and prints a compact, human-
// readable trading decision journal (newest first).
//
// The Guardrail event log is the append-only source of truth for every decision
// the agent makes: regime classifications, target computations, risk verdicts,
// orders, quotes, and reconciliations (see docs/EXPLAINABILITY.md). This script
// turns that raw event stream into a one-line-per-entry journal a reviewer can
// skim.
//
// Offline-safe: if the API is unreachable it prints a friendly notice and exits
// 0 (never a stack trace, never a nonzero exit), so it is safe to run before the
// API is up.
//
// Run from the repo root (start the API first: `cargo run -p guardrail-api`):
//   node clients/examples/journal_export.mjs
//
// Configure the target with GUARDRAIL_BASE_URL (default http://localhost:8080).
// Limit the number of rows with GUARDRAIL_JOURNAL_LIMIT (default 25).

const DEFAULT_BASE_URL = "http://localhost:8080";
const DEFAULT_LIMIT = 25;

function parseLimit(raw) {
  const n = Number.parseInt(raw ?? "", 10);
  if (Number.isFinite(n) && n > 0) return Math.min(n, 500);
  return DEFAULT_LIMIT;
}

async function fetchEvents(baseUrl, timeoutMs = 8000) {
  const fetchImpl = globalThis.fetch;
  if (!fetchImpl) {
    throw new Error("No fetch implementation available (need Node 18+).");
  }
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetchImpl(`${baseUrl.replace(/\/+$/, "")}/events`, {
      headers: { Accept: "application/json" },
      signal: controller.signal,
    });
    if (!res.ok) {
      throw new Error(`GET /events failed: ${res.status} ${res.statusText}`);
    }
    return res.json();
  } finally {
    clearTimeout(timer);
  }
}

// Events are loose JSON; pull the most useful fields defensively.
function pick(obj, keys) {
  for (const k of keys) {
    if (obj && obj[k] !== undefined && obj[k] !== null && obj[k] !== "") {
      return obj[k];
    }
  }
  return undefined;
}

function eventKind(ev) {
  return (
    pick(ev, ["kind", "type", "event_type", "event", "name"]) ?? "event"
  );
}

function eventTime(ev) {
  return pick(ev, ["timestamp", "ts", "time", "created_at", "occurred_at"]);
}

// Build a short, kind-aware one-liner from whatever payload fields exist.
function summarize(ev) {
  const payload =
    ev && typeof ev.payload === "object" && ev.payload !== null ? ev.payload : ev;
  const interesting = [
    "regime",
    "decision",
    "verdict",
    "action",
    "symbol",
    "asset",
    "side",
    "status",
    "reason",
    "amount_usd",
    "nav_usd",
    "kill_switch",
  ];
  const parts = [];
  for (const key of interesting) {
    const v = payload?.[key];
    if (v !== undefined && v !== null && v !== "" && typeof v !== "object") {
      parts.push(`${key}=${v}`);
    }
  }
  if (parts.length > 0) return parts.join(" ");
  // Fall back to listing payload keys so the row is never empty.
  const keys = payload && typeof payload === "object" ? Object.keys(payload) : [];
  return keys.length ? `fields: ${keys.slice(0, 6).join(", ")}` : "(no fields)";
}

function printJournal(events, limit) {
  if (!Array.isArray(events) || events.length === 0) {
    console.log("No events recorded yet. The decision journal is empty.");
    return;
  }
  const rows = events.slice(0, limit);
  console.log(`Decision journal -- ${rows.length} of ${events.length} event(s), newest first:\n`);
  let idx = 1;
  for (const ev of rows) {
    const ts = eventTime(ev) ?? "(no time)";
    const kind = String(eventKind(ev)).padEnd(22);
    const num = String(idx).padStart(3, " ");
    console.log(`${num}. ${ts}  ${kind}  ${summarize(ev)}`);
    idx += 1;
  }
  if (events.length > rows.length) {
    console.log(
      `\n... ${events.length - rows.length} older event(s) not shown ` +
        `(raise GUARDRAIL_JOURNAL_LIMIT to see more).`,
    );
  }
}

async function main() {
  const baseUrl = process.env.GUARDRAIL_BASE_URL ?? DEFAULT_BASE_URL;
  const limit = parseLimit(process.env.GUARDRAIL_JOURNAL_LIMIT);

  console.log(`Guardrail decision journal export -> ${baseUrl}\n`);

  let data;
  try {
    data = await fetchEvents(baseUrl);
  } catch (err) {
    const reason = err instanceof Error ? err.message : String(err);
    console.log("Notice: could not fetch the event log from the Guardrail API.");
    console.log(`  Is it running at ${baseUrl}? Start it with: cargo run -p guardrail-api`);
    console.log(`  Reason: ${reason}`);
    process.exit(0);
  }

  if (data && data.error) {
    console.log(`Notice: the API returned an error reading the event log: ${data.error}`);
    process.exit(0);
  }

  const events = Array.isArray(data) ? data : data?.events;
  printJournal(events, limit);
  console.log("\nDone.");
}

main();
