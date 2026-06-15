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
//
// Flags (any position):
//   --base URL   API base URL (default $GUARDRAIL_BASE_URL or http://localhost:8080)
//   --json       emit machine-readable JSON instead of a table

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
}
declare const process: ProcessLike;

// Only two process exit codes. Operational failures (API down, decode errors)
// deliberately still exit 0 so the tool is safe to run offline; only a usage
// mistake exits non-zero.
const EXIT_OK = 0;
const EXIT_USAGE = 2;

const DEFAULT_BASE_URL = "http://localhost:8080";
const REQUEST_TIMEOUT_MS = 5000;

/** Parsed argv: subcommand, common flags, and any leftover positionals. */
interface ParsedArgs {
  command: string;
  base: string;
  json: boolean;
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
    positionals: [],
  };
  const rest: string[] = [];

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--json") {
      result.json = true;
    } else if (arg === "--base") {
      const next = argv[i + 1];
      if (next == null) {
        throw new Error("--base requires a URL argument");
      }
      result.base = next;
      i++;
    } else if (arg.startsWith("--base=")) {
      result.base = arg.slice("--base=".length);
    } else {
      rest.push(arg);
    }
  }

  result.command = rest.shift() ?? "";
  result.positionals = rest;
  return result;
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
  help       show this help

Common flags:
  --base URL   API base URL (default $GUARDRAIL_BASE_URL or ${DEFAULT_BASE_URL})
  --json       emit JSON instead of a table

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
