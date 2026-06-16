import { getJsonOrNull } from "../../lib/api";

type Num = string | number;

interface Breakdown {
  gas_usd?: Num;
  slippage_usd?: Num;
  fee_usd?: Num;
  total_usd?: Num;
  effective_price?: Num;
  total_cost_fraction?: Num;
}

interface FeesResponse {
  side?: string;
  breakdown?: Breakdown;
}

const ROWS: Array<[keyof Breakdown, string]> = [
  ["gas_usd", "Gas (USD)"],
  ["slippage_usd", "Slippage / price impact (USD)"],
  ["fee_usd", "Protocol fee (USD)"],
  ["total_usd", "Total cost (USD)"],
  ["effective_price", "Effective price"],
  ["total_cost_fraction", "Total cost (fraction of notional)"],
];

export default async function FeesPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const p = await searchParams;
  const notional = p.notional_usd ?? "10000";
  const quantity = p.quantity ?? "5";
  const side = p.side === "sell" ? "sell" : "buy";
  const qs = new URLSearchParams({ notional_usd: notional, quantity, side }).toString();
  const data = await getJsonOrNull<FeesResponse>(`/fees?${qs}`);
  const b = data?.breakdown;

  return (
    <main className="grid">
      <section className="card">
        <h1>Swap Cost Estimator</h1>
        <p>
          All-in cost of a swap via the <code>fee-model</code> crate
          (<code>GET /fees</code>): gas + constant-product price impact + linear
          slippage + protocol fee. Drive it with{" "}
          <code>?notional_usd=&amp;quantity=&amp;side=buy|sell</code>.
        </p>
        <p>
          <strong>side</strong> {side} · <strong>notional</strong> ${notional} ·{" "}
          <strong>quantity</strong> {quantity}
        </p>
        {!b ? (
          <p>Unavailable (is the API running?).</p>
        ) : (
          <table>
            <tbody>
              {ROWS.map(([key, label]) => (
                <tr key={key}>
                  <td>{label}</td>
                  <td>
                    <strong>{b[key] !== undefined ? String(b[key]) : "—"}</strong>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
