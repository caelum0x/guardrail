import { ApiClient } from "./api.js";
import { bad, heading, kvTable, ok, table, warn } from "./render.js";
import type { EventsResponse, Health, Json, Regime, Verify } from "./types.js";

const SIMPLE = ["portfolio", "trades", "risk", "signals", "proof", "cockpit"] as const;
const COMMANDS = ["status", "regime", "verify", "events", "watch", ...SIMPLE] as const;

interface Args {
  api: string;
  json: boolean;
  command: string;
  rest: string[];
}

function parseArgs(argv: string[]): Args {
  let api = process.env.GUARDRAIL_API ?? "http://127.0.0.1:8080";
  let json = false;
  let command = "";
  const rest: string[] = [];
  for (const a of argv) {
    if (a === "--json") json = true;
    else if (a.startsWith("--api=")) api = a.slice("--api=".length);
    else if (a.startsWith("--")) continue;
    else if (!command) command = a;
    else rest.push(a);
  }
  return { api, json, command, rest };
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function run(): Promise<number> {
  const { api, json, command, rest } = parseArgs(process.argv.slice(2));
  const client = new ApiClient(api);

  if (!command || command === "help") {
    console.log(heading("guardrail-term") + " — read-only Guardrail terminal client\n");
    console.log("usage: guardrail-term [--api=URL] [--json] <command> [args]");
    console.log(`commands: ${COMMANDS.join(" ")}`);
    console.log(`api: ${api} (override with --api=URL or $GUARDRAIL_API)`);
    return command ? 0 : 2;
  }

  if (command === "watch") {
    const seconds = Number(rest[0] ?? "5") || 5;
    console.log(`watching /regime every ${seconds}s (ctrl-c to stop)`);
    for (;;) {
      try {
        const r = await client.get<Regime>("/regime");
        console.log(`${new Date().toISOString()}  regime=${r.regime ?? "?"} exposure=${r.exposure ?? "?"}`);
      } catch (err) {
        console.log(`${new Date().toISOString()}  ${bad(String(err))}`);
      }
      await sleep(seconds * 1000);
    }
  }

  try {
    const path = command === "verify" ? "/proof/verify" : command === "status" ? "/health" : `/${command}`;
    const data = await client.get<Json>(path);

    if (json) {
      console.log(JSON.stringify(data, null, 2));
      return 0;
    }

    renderCommand(command, data);
    return 0;
  } catch (err) {
    console.error(bad(`error: ${String(err)}`));
    return 1;
  }
}

function renderCommand(command: string, data: Json): void {
  switch (command) {
    case "status": {
      const h = data as Health;
      console.log(kvTable([
        ["status", String(h.status ?? "?")],
        ["events", String(h.events ?? "?")],
      ]));
      break;
    }
    case "regime": {
      const r = data as Regime;
      console.log(kvTable([
        ["regime", String(r.regime ?? "?")],
        ["exposure", String(r.exposure ?? "?")],
      ]));
      break;
    }
    case "verify": {
      const v = data as Verify;
      console.log(v.passed ? ok("PASSED") : bad("FAILED"));
      const rows = (v.checks ?? []).map((c) => [
        c.status === "pass" ? ok(c.status) : c.status === "skipped" ? warn(c.status) : bad(c.status),
        c.name,
        c.detail,
      ]);
      if (rows.length) console.log(table(["status", "check", "detail"], rows));
      break;
    }
    case "events": {
      const e = data as EventsResponse;
      const rows = (e.events ?? []).slice(0, 15).map((ev) => [
        String(ev.timestamp ?? ""),
        String(ev.event_type ?? ""),
      ]);
      console.log(table(["timestamp", "event"], rows));
      break;
    }
    default: {
      // Top-level key summary for portfolio/trades/risk/signals/proof/cockpit.
      const rows: Array<[string, string]> = Object.entries(data).map(([k, v]) => [
        k,
        Array.isArray(v) ? `[${v.length} items]` : typeof v === "object" && v !== null ? "{…}" : String(v),
      ]);
      console.log(kvTable(rows));
    }
  }
}

run().then((code) => process.exit(code));
