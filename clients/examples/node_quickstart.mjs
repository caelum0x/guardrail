#!/usr/bin/env node
// End-to-end quickstart for the Guardrail TypeScript SDK surface, runnable
// directly under Node 18+ with zero dependencies.
//
// The TS SDK source lives at ../typescript/src/index.ts and cannot be imported
// by node without a `tsc` build step. To keep this example dependency-free and
// instantly runnable, we mirror the SDK's method set here using the global
// `fetch` (the exact transport the SDK itself uses). The method names, routes,
// and query parameters match clients/typescript/src/index.ts one-for-one.
//
// Guided sequence (a concise summary of each is printed):
//   1. health()        -- API + database status
//   2. compilePolicy() -- compile a natural-language mandate into a policy hash
//   3. backtest()      -- strategy vs benchmark over 60 steps
//   4. walkforward()   -- rolling out-of-sample windows
//   5. regime()        -- current market regime
//   6. compete()       -- competition status
//
// A down / unreachable API prints a friendly notice and exits 0 (never a stack
// trace).
//
// Run from the repo root (start the API first: `cargo run -p guardrail-api`):
//   node clients/examples/node_quickstart.mjs
//
// Configure the target with GUARDRAIL_BASE_URL (default http://localhost:8080).

const DEFAULT_BASE_URL = "http://localhost:8080";

// Minimal mirror of clients/typescript/src/index.ts GuardrailClient. Only the
// methods exercised by this quickstart are implemented; they share the same
// routes and query semantics as the published SDK.
class GuardrailClient {
  constructor(options = {}) {
    this.baseUrl = (options.baseUrl ?? DEFAULT_BASE_URL).replace(/\/+$/, "");
    const f = options.fetchImpl ?? globalThis.fetch;
    if (!f) {
      throw new Error("No fetch implementation available (need Node 18+).");
    }
    this.fetchImpl = f.bind(globalThis);
  }

  async getJson(path) {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`, {
      headers: { Accept: "application/json" },
    });
    if (!res.ok) {
      throw new Error(`GET ${path} failed: ${res.status} ${res.statusText}`);
    }
    return res.json();
  }

  health() {
    return this.getJson("/health");
  }

  backtest(params = {}) {
    const q = new URLSearchParams();
    if (params.steps != null) q.set("steps", String(params.steps));
    if (params.fearGreed != null) q.set("fear_greed", String(params.fearGreed));
    if (params.preset) q.set("preset", params.preset);
    return this.getJson(`/backtest?${q.toString()}`);
  }

  walkforward(params = {}) {
    const q = new URLSearchParams();
    if (params.windows != null) q.set("windows", String(params.windows));
    if (params.steps != null) q.set("steps", String(params.steps));
    if (params.preset) q.set("preset", params.preset);
    return this.getJson(`/walkforward?${q.toString()}`);
  }

  regime() {
    return this.getJson("/regime");
  }

  compete() {
    return this.getJson("/compete");
  }

  compilePolicy(mandate) {
    return this.getJson(`/policy/compile?mandate=${encodeURIComponent(mandate)}`);
  }
}

function fmt(value) {
  return value === undefined || value === null ? "n/a" : String(value);
}

function summarizeHealth(d) {
  return `ok=${fmt(d.ok)} events_visible=${fmt(d.events_visible)}`;
}

function summarizePolicy(d) {
  if (d.error) return `error=${d.error}`;
  return `hash=${fmt(d.hash)}`;
}

function summarizeBacktest(d) {
  const m = d.metrics ?? {};
  return (
    `steps=${fmt(d.steps)} ` +
    `final_nav_usd=${fmt(d.final_nav_usd)} ` +
    `total_return_pct=${fmt(m.total_return_pct)} ` +
    `max_drawdown_pct=${fmt(m.max_drawdown_pct)} ` +
    `excess_return_pct=${fmt(d.excess_return_pct)}`
  );
}

function summarizeWalkforward(d) {
  const windows = d.windows ?? [];
  const agg = d.aggregate ?? {};
  return (
    `windows=${windows.length} ` +
    `mean_excess_pct=${fmt(agg.mean_excess_pct)} ` +
    `positive_windows=${fmt(agg.positive_windows)}`
  );
}

function summarizeOpen(d, preferred) {
  for (const key of preferred) {
    if (key in d) return `${key}=${fmt(d[key])}`;
  }
  const keys = Object.keys(d).sort().join(", ");
  return `keys: ${keys || "(empty)"}`;
}

async function main() {
  const baseUrl = process.env.GUARDRAIL_BASE_URL ?? DEFAULT_BASE_URL;
  const client = new GuardrailClient({ baseUrl });

  console.log(`Guardrail TypeScript SDK quickstart -> ${baseUrl}\n`);

  try {
    console.log("[1/6] health()");
    console.log("      " + summarizeHealth(await client.health()));

    const mandate = "Trade CAKE max drawdown 20% kill switch 25%";
    console.log(`\n[2/6] compilePolicy(${JSON.stringify(mandate)})`);
    console.log("      " + summarizePolicy(await client.compilePolicy(mandate)));

    console.log("\n[3/6] backtest({ steps: 60, fearGreed: 70, preset: 'balanced' })");
    console.log(
      "      " +
        summarizeBacktest(
          await client.backtest({ steps: 60, fearGreed: 70, preset: "balanced" }),
        ),
    );

    console.log("\n[4/6] walkforward()");
    console.log("      " + summarizeWalkforward(await client.walkforward()));

    console.log("\n[5/6] regime()");
    console.log(
      "      " + summarizeOpen(await client.regime(), ["regime", "label", "state", "name"]),
    );

    console.log("\n[6/6] compete()");
    console.log(
      "      " +
        summarizeOpen(await client.compete(), ["status", "competition", "rank", "name"]),
    );
  } catch (err) {
    const reason = err instanceof Error ? err.message : String(err);
    console.log("\nNotice: could not complete the sequence against the Guardrail API.");
    console.log(`  Is it running at ${baseUrl}? Start it with: cargo run -p guardrail-api`);
    console.log(`  Reason: ${reason}`);
    process.exit(0);
  }

  console.log("\nDone. All calls completed successfully.");
}

main();
