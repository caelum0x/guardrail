// Runnable example: offline proof verification + event-journal walk.
//
// This example is fully OFFLINE and deterministic. It does not require a running
// API: the proof is the bundled sample fixture, and the journal is served by a
// tiny in-memory `fetch` stub passed to the client. It exits 0 on success and a
// non-zero code on any verification failure, so it can be wired into CI.
//
// Run with any TS runner, e.g.:
//   npx tsx examples/verify-and-journal.ts
//   node --experimental-strip-types examples/verify-and-journal.ts   (Node 22.6+)

import { GuardrailClient } from "../src/index.js";
import type { ProofResponse, StoredEvent } from "../src/index.js";
import { renderReport, verifyProof } from "../src/index.js";

// Minimal structural declaration of Node's `process.exit` so this example
// type-checks without depending on `@types/node` (this client is dependency-free).
declare const process: { exit(code?: number): never };

// A self-contained /proof envelope identical in shape to the agent's output and
// the bundled clients/proof-verifier/sample_proof.json fixture. Hashes here are
// illustrative placeholders; the verifier still validates wallet/url/contract
// formats and re-derives report_hash from the core fields.
const SAMPLE_PROOF: ProofResponse = {
  agent: "guardrail-alpha",
  registration_tx: "0x4ab19558e8b5abc54636307c5c6a124ce4d7610f37a43f8d391a165f2125c7b1",
  latest_report: {
    run_id: "run_58680f6fbebd4dbea653d1a69f307b14",
    cycles: 4,
    final_nav_usd: "9989.75",
    total_drawdown_pct: "0.1025",
    events: 75,
    agent_id: "e38b86d49c975f0b3428d973141b89cda5281c2b330bfc29d9c418fb078012a4",
    wallet_address: "0xA9e5C0FfEe0000000000000000000000000A1b2C3",
    policy_hash: "a25d3a24f541ed163a8e561a10ef838694362769597a1be76799200408acc1d2",
    report_hash: "e34f0bc422b37fab664390e057b7b57123999d10acde18fcd49480a5c6401d70",
    address_url: "https://bscscan.com/address/0xA9e5C0FfEe0000000000000000000000000A1b2C3",
    registration_tx_url:
      "https://bscscan.com/tx/0x4ab19558e8b5abc54636307c5c6a124ce4d7610f37a43f8d391a165f2125c7b1",
  },
  run_report: {
    run_id: "run_58680f6fbebd4dbea653d1a69f307b14",
    wallet_address: "0xA9e5C0FfEe0000000000000000000000000A1b2C3",
    policy_hash: "a25d3a24f541ed163a8e561a10ef838694362769597a1be76799200408acc1d2",
  },
  source_event_id: "evt_sample_0001",
};

const SAMPLE_EVENTS: StoredEvent[] = [
  {
    id: "evt_sample_0001",
    run_id: "run_58680f6fbebd4dbea653d1a69f307b14",
    timestamp: "2026-06-13T12:00:00Z",
    event_type: "MarketSnapshotReceived",
    payload_json: { symbol: "WBNB", price_usd: "612.40" },
  },
  {
    id: "evt_sample_0002",
    run_id: "run_58680f6fbebd4dbea653d1a69f307b14",
    timestamp: "2026-06-13T12:00:05Z",
    event_type: "RiskApproved",
    payload_json: { symbol: "WBNB", weight_pct: "12.5" },
  },
];

// An offline `fetch` stub that serves only the routes this example reads.
const offlineFetch: typeof fetch = async (input) => {
  const url = typeof input === "string" ? input : input.toString();
  if (url.endsWith("/events")) {
    return new Response(JSON.stringify({ events: SAMPLE_EVENTS }), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  }
  return new Response(JSON.stringify({ error: "not found" }), { status: 404 });
};

async function main(): Promise<number> {
  // 1. Verify the proof entirely offline (re-derives report_hash via SHA-256).
  const result = await verifyProof(SAMPLE_PROOF);
  console.log(renderReport(result, "examples/SAMPLE_PROOF"));

  if (!result.passed) {
    console.error("\nProof verification FAILED");
    return 1;
  }

  // 2. Walk the event journal through the typed client (offline fetch stub).
  const client = new GuardrailClient({
    baseUrl: "http://localhost:8080",
    fetchImpl: offlineFetch,
  });
  const { events } = await client.events();
  console.log(`\nJournal: ${events.length} event(s)`);
  for (const event of events) {
    console.log(`  [${event.timestamp}] ${event.event_type} (${event.id})`);
  }

  console.log("\nOK: proof verified and journal read offline.");
  return 0;
}

main()
  .then((code) => process.exit(code))
  .catch((error: unknown) => {
    console.error("Example crashed:", error);
    process.exit(1);
  });
