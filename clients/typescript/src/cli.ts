#!/usr/bin/env node
// guardrail — a dependency-free operator CLI for the Guardrail Alpha API.
//
// This mirrors the Go `guardrailctl` (clients/go/cmd/guardrailctl): it is
// offline-safe by design — every subcommand prints a useful line and exits 0
// even when the API is unreachable, so it is harmless to run in CI or against a
// stopped backend. It uses only Node built-ins plus the existing SDK client
// (which itself depends only on the global `fetch`, available in Node 18+).
//
// Subcommands:
//   status     fetch /compete + /readiness (with a /regime status line)
//   regime     show the current market regime
//   journal    show a compact per-cycle decision journal
//   ensemble   show the current regime and per-skill ensemble weights
//   skills     show the Skill catalog, or one skill's detail (skills ID)
//   verify     show the server-side /proof/verify pass/fail table
//   snapshots  show the latest run summary and per-asset latest-price sample
//   watch      poll /compete + /regime on an interval, refreshing a status line
//
// Flags (any position):
//   --base URL       API base URL (default $GUARDRAIL_BASE_URL or http://localhost:8080)
//   --json           emit machine-readable JSON instead of a table
//   --interval SECS  watch poll interval in seconds (default 5, min 1)
//   --once           watch: print a single tick and exit

import { GuardrailClient } from "./index.js";
import {
  renderEnsemble,
  renderJournal,
  renderSkillCatalog,
  renderSkillDetail,
  renderSnapshots,
  renderStatusLine,
  renderVerify,
  table,
} from "./cli-render.js";

// Minimal structural view of Node's global `process`, declared locally so this
// client stays dependency-free (it ships no `@types/node`). Only the members the
// CLI actually uses are typed; the runtime object provides the rest.
interface ProcessLike {
  argv: string[];
  env: Record<string, string | undefined>;
  exitCode?: number;
  stdout: { write(chunk: string): boolean };
  stderr: { write(chunk: string): boolean };
  /** Register a one-shot-friendly signal listener (used to stop `watch`). */
  on(event: "SIGINT" | "SIGTERM", listener: () => void): void;
  /** Remove a previously registered signal listener. */
  off(event: "SIGINT" | "SIGTERM", listener: () => void): void;
}
declare const process: ProcessLike;

// Node/browser timer globals, typed minimally so the client stays free of
// `@types/node`. We only use the handle as an opaque token passed to clearX.
declare function setInterval(handler: () => void, ms: number): unknown;
declare function clearInterval(handle: unknown): void;

// Only two process exit codes. Operational failures (API down, decode errors)
// deliberately still exit 0 so the tool is safe to run offline; only a usage
// mistake exits non-zero.
const EXIT_OK = 0;
const EXIT_USAGE = 2;

const DEFAULT_BASE_URL = "http://localhost:8080";
const REQUEST_TIMEOUT_MS = 5000;

// Default + floor for the `watch` poll interval. The floor guards against a
// zero or absurdly small interval that would busy-loop the API (mirrors the Go
// guardrailctl `minInterval`).
const DEFAULT_INTERVAL_SEC = 5;
const MIN_INTERVAL_SEC = 1;

/** Parsed argv: subcommand, common flags, and any leftover positionals. */
interface ParsedArgs {
  command: string;
  base: string;
  json: boolean;
  /** `watch` poll interval in seconds (already clamped to >= MIN_INTERVAL_SEC). */
  interval: number;
  /** `watch`: print a single tick and exit. */
  once: boolean;
  positionals: string[];
}

/**
 * Parse argv (excluding `node` + script). The first non-flag token is the
 * subcommand; remaining non-flag tokens become positionals. `--base` accepts
 * either `--base=URL` or `--base URL`.
 */
function parseArgs(argv: string[]): ParsedArgs {
  const envBase = process.env.GUARDRAIL_BASE_URL;
  const result: ParsedArgs = {
    command: "",
    base: envBase && envBase.trim() !== "" ? envBase : DEFAULT_BASE_URL,
    json: false,
    interval: DEFAULT_INTERVAL_SEC,
    once: false,
    positionals: [],
  };
  const rest: string[] = [];

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--json") {
      result.json = true;
    } else if (arg === "--once") {
      result.once = true;
    } else if (arg === "--base") {
      const next = argv[i + 1];
      if (next == null) {
        throw new Error("--base requires a URL argument");
      }
      result.base = next;
      i++;
    } else if (arg.startsWith("--base=")) {
      result.base = arg.slice("--base=".length);
    } else if (arg === "--interval") {
      const next = argv[i + 1];
      if (next == null) {
        throw new Error("--interval requires a seconds argument");
      }
      result.interval = parseInterval(next);
      i++;
    } else if (arg.startsWith("--interval=")) {
      result.interval = parseInterval(arg.slice("--interval=".length));
    } else {
      rest.push(arg);
    }
  }

  result.command = rest.shift() ?? "";
  result.positionals = rest;
  return result;
}

/**
 * Parse a `--interval` value to an integer number of seconds, clamped to
 * `MIN_INTERVAL_SEC`. A non-numeric value is a usage error (thrown), matching
 * how `--base` rejects a missing argument.
 */
function parseInterval(raw: string): number {
  const n = Number(raw);
  if (!Number.isFinite(n)) {
    throw new Error(`--interval requires a number of seconds, got "${raw}"`);
  }
  const secs = Math.floor(n);
  return secs < MIN_INTERVAL_SEC ? MIN_INTERVAL_SEC : secs;
}

/** Build an SDK client whose fetch enforces a short per-call timeout so an
 * unreachable host fails fast rather than hanging. */
function newClient(base: string): GuardrailClient {
  const timedFetch: typeof fetch = (input, init) => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
    return fetch(input, { ...init, signal: controller.signal }).finally(() =>
      clearTimeout(timer),
    );
  };
  return new GuardrailClient({ baseUrl: base, fetchImpl: timedFetch });
}

/** Narrow an unknown thrown value to a printable message. */
function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

/** Format a one-line, non-fatal notice for a failed API call. */
function unavailable(label: string, error: unknown): string {
  return `${label}: unavailable: ${errorMessage(error)}`;
}

function printJSON(value: unknown): void {
  process.stdout.write(`${JSON.stringify(value, null, 2)}\n`);
}

function println(text: string): void {
  process.stdout.write(`${text}\n`);
}

// --- Subcommands ----------------------------------------------------------

async function cmdStatus(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  const stamp = new Date().toISOString().slice(11, 19);

  const [regime, compete, readiness] = await Promise.all([
    client.regime().catch(() => null),
    client.compete().catch(() => null),
    client.readiness().catch(() => null),
  ]);

  if (args.json) {
    printJSON({
      time: stamp,
      regime: regime ?? { status: "offline" },
      compete: compete ?? { status: "offline" },
      readiness: readiness ?? { status: "offline" },
    });
    return EXIT_OK;
  }

  println(renderStatusLine(stamp, regime, compete));
  println("");
  if (readiness == null) {
    println("readiness: offline");
    return EXIT_OK;
  }
  println(`readiness: ${readiness.status}  (${readiness.blocking} blocking)`);
  if (readiness.checks.length > 0) {
    const rows = [["STATUS", "CHECK", "DETAIL"]];
    for (const c of readiness.checks) {
      rows.push([c.status.toUpperCase(), c.id, c.detail]);
    }
    println(table(rows));
  }
  return EXIT_OK;
}

async function cmdRegime(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  let regime;
  try {
    regime = await client.regime();
  } catch (error) {
    println(unavailable("regime", error));
    return EXIT_OK;
  }
  if (args.json) {
    printJSON(regime);
    return EXIT_OK;
  }
  println(`regime: ${regime.regime}`);
  println(`  exposure multiplier: ${regime.exposure_multiplier}`);
  println(
    `  inputs: f/g=${regime.inputs.fear_greed} breadth=${regime.inputs.breadth_pct}% ` +
      `btc_dom=${regime.inputs.btc_dominance_pct}% median_24h=${regime.inputs.median_24h_return}`,
  );
  return EXIT_OK;
}

async function cmdJournal(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  let resp;
  try {
    resp = await client.journal();
  } catch (error) {
    println(unavailable("journal", error));
    return EXIT_OK;
  }
  if (args.json) {
    printJSON(resp);
    return EXIT_OK;
  }
  println(renderJournal(resp));
  return EXIT_OK;
}

async function cmdEnsemble(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  let resp;
  try {
    resp = await client.ensemble();
  } catch (error) {
    println(unavailable("ensemble", error));
    return EXIT_OK;
  }
  if (args.json) {
    printJSON(resp);
    return EXIT_OK;
  }
  println(renderEnsemble(resp));
  return EXIT_OK;
}

async function cmdSkills(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  const id = args.positionals[0];

  if (id != null && id !== "") {
    let resp;
    try {
      resp = await client.skillById(id);
    } catch (error) {
      println(unavailable("skills", error));
      return EXIT_OK;
    }
    if (args.json) {
      printJSON(resp);
      return EXIT_OK;
    }
    println(renderSkillDetail(id, resp));
    return EXIT_OK;
  }

  let resp;
  try {
    resp = await client.skills();
  } catch (error) {
    println(unavailable("skills", error));
    return EXIT_OK;
  }
  if (args.json) {
    printJSON(resp);
    return EXIT_OK;
  }
  println(renderSkillCatalog(resp));
  return EXIT_OK;
}

async function cmdVerify(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  let resp;
  try {
    resp = await client.proofVerify();
  } catch (error) {
    println(unavailable("verify", error));
    return EXIT_OK;
  }
  if (args.json) {
    printJSON(resp);
    return EXIT_OK;
  }
  println(renderVerify(resp));
  return EXIT_OK;
}

async function cmdSnapshots(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);
  let resp;
  try {
    resp = await client.snapshots();
  } catch (error) {
    println(unavailable("snapshots", error));
    return EXIT_OK;
  }
  if (args.json) {
    printJSON(resp);
    return EXIT_OK;
  }
  println(renderSnapshots(resp));
  return EXIT_OK;
}

// --- watch ----------------------------------------------------------------

// Width the in-place status line is padded to so a shorter line fully clears a
// longer previous one (mirrors the Go guardrailctl `%-110s`).
const WATCH_LINE_WIDTH = 110;

/** Fetch /regime + /compete once and render/emit a single status tick. */
async function watchTick(client: GuardrailClient, asJSON: boolean): Promise<void> {
  const stamp = new Date().toISOString().slice(11, 19);
  const [regime, compete] = await Promise.all([
    client.regime().catch(() => null),
    client.compete().catch(() => null),
  ]);

  if (asJSON) {
    // One discrete JSON object per tick so the stream stays line-parseable.
    process.stdout.write(
      `${JSON.stringify({
        time: stamp,
        regime: regime ?? { status: "offline" },
        compete: compete ?? { status: "offline" },
      })}\n`,
    );
    return;
  }

  // \r returns to column 0; padding clears any longer previous line so the
  // status appears to refresh in place.
  process.stdout.write(`\r${renderStatusLine(stamp, regime, compete).padEnd(WATCH_LINE_WIDTH)}`);
}

/**
 * Poll /compete + /regime on an interval and print a refreshing one-line
 * status. `--once` prints a single tick and exits; otherwise it loops until
 * SIGINT/SIGTERM. Always resolves with EXIT_OK so it stays offline-safe.
 */
async function cmdWatch(args: ParsedArgs): Promise<number> {
  const client = newClient(args.base);

  // First tick immediately so the operator sees output without waiting.
  await watchTick(client, args.json);

  if (args.once) {
    // Terminate the in-place status line so the shell prompt starts cleanly.
    if (!args.json) println("");
    return EXIT_OK;
  }

  return new Promise<number>((resolve) => {
    let stopped = false;

    const timer = setInterval(() => {
      // A tick can only fail catastrophically (not an API error, which
      // watchTick already folds into "offline"); swallow to stay offline-safe.
      void watchTick(client, args.json).catch(() => undefined);
    }, args.interval * 1000);

    const stop = (): void => {
      if (stopped) return;
      stopped = true;
      clearInterval(timer);
      process.off("SIGINT", stop);
      process.off("SIGTERM", stop);
      // Move past the in-place status line before exiting.
      if (!args.json) println("");
      resolve(EXIT_OK);
    };

    process.on("SIGINT", stop);
    process.on("SIGTERM", stop);
  });
}

// --- Help + dispatch ------------------------------------------------------

const USAGE = `guardrail — operator CLI for the Guardrail Alpha API

Usage:
  guardrail <command> [flags]

Commands:
  status     show /compete + /readiness with a /regime status line
  regime     show the current market regime
  journal    show a compact per-cycle decision journal
  ensemble   show the current regime and per-skill ensemble weights
  skills     show the Skill catalog, or one skill's detail (skills ID)
  verify     show the server-side /proof/verify pass/fail table
  snapshots  show the latest run summary and per-asset latest-price sample
  watch      poll /compete + /regime on an interval, refreshing a status line
  help       show this help

Common flags:
  --base URL   API base URL (default $GUARDRAIL_BASE_URL or ${DEFAULT_BASE_URL})
  --json       emit JSON instead of a table

watch flags:
  --interval N  poll interval in seconds (default ${DEFAULT_INTERVAL_SEC}, min ${MIN_INTERVAL_SEC})
  --once        print a single status tick and exit
  (--json emits one JSON object per tick; Ctrl-C stops cleanly)

All commands are offline-safe: they print a notice and exit 0 when the API is
unreachable.`;

/** Dispatch a parsed command to its handler. Pure of process.exit. */
async function dispatch(args: ParsedArgs): Promise<number> {
  switch (args.command) {
    case "status":
      return cmdStatus(args);
    case "regime":
      return cmdRegime(args);
    case "journal":
      return cmdJournal(args);
    case "ensemble":
      return cmdEnsemble(args);
    case "skills":
      return cmdSkills(args);
    case "verify":
      return cmdVerify(args);
    case "snapshots":
      return cmdSnapshots(args);
    case "watch":
      return cmdWatch(args);
    case "help":
    case "-h":
    case "--help":
      println(USAGE);
      return EXIT_OK;
    case "":
      process.stderr.write(`${USAGE}\n`);
      return EXIT_USAGE;
    default:
      process.stderr.write(`guardrail: unknown command "${args.command}"\n\n${USAGE}\n`);
      return EXIT_USAGE;
  }
}

async function main(): Promise<number> {
  let args: ParsedArgs;
  try {
    args = parseArgs(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`guardrail: ${errorMessage(error)}\n\n${USAGE}\n`);
    return EXIT_USAGE;
  }
  return dispatch(args);
}

main()
  .then((code) => {
    process.exitCode = code;
  })
  .catch((error: unknown) => {
    // A truly unexpected failure (not an API call) still should not hard-crash
    // the offline-safe contract for known commands; surface it and exit 0.
    process.stdout.write(`guardrail: ${errorMessage(error)}\n`);
    process.exitCode = EXIT_OK;
  });
