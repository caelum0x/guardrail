/**
 * Demonstrate the Guardrail quant endpoints via the TypeScript SDK.
 *
 * Run a Guardrail API locally (`cargo run -p guardrail-api`), then build the
 * SDK (`npm run build`) and run:
 *
 *     node dist-examples/quant-tools.js
 *
 * or with a TS runner. Every call is read-only. GUARDRAIL_API overrides the host.
 */
import { GuardrailClient } from "../src/index.js";

// Minimal structural declaration of Node's `process` so this example type-checks
// without depending on `@types/node` (this client is dependency-free).
declare const process: { env: Record<string, string | undefined>; exitCode?: number };

async function main(): Promise<void> {
  const client = new GuardrailClient({
    baseUrl: process.env.GUARDRAIL_API ?? "http://127.0.0.1:8080",
  });

  const rsi = await client.ta({
    indicator: "rsi",
    series: [44, 44.3, 44.1, 43.6, 44.3, 44.8, 45.1, 45.4, 45.8, 46.0],
    period: 5,
  });
  console.log("RSI:", rsi.result ?? rsi);

  const cost = await client.fees({ notionalUsd: 25000, quantity: 12, side: "buy" });
  console.log("swap cost:", cost.breakdown ?? cost);

  const size = await client.sizer({ method: "kelly", win_prob: 0.6, odds: 1.5 });
  console.log("kelly size:", size.output ?? size);

  const pnl = await client.pnl("CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3");
  console.log("pnl:", (pnl.report as Record<string, unknown>)?.total ?? pnl);
}

main().catch((err: unknown) => {
  console.error("error (is the API running?):", err);
  process.exitCode = 1;
});
