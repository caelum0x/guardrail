import { getJsonOrNull } from "../../lib/api";

type Num = string | number;

interface PnLRow {
  symbol: string;
  position: Num;
  avg_cost: Num;
  realized: Num;
  unrealized: Num;
  fees: Num;
  total: Num;
}
interface PnlResponse {
  fills?: string;
  report?: {
    by_symbol?: PnLRow[];
    total?: { realized: Num; unrealized: Num; fees: Num; total: Num };
  };
  error?: string;
}

export default async function PnlPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const p = await searchParams;
  const q = new URLSearchParams();
  if (p.fills) q.set("fills", p.fills);
  if (p.marks) q.set("marks", p.marks);
  const qs = q.toString();
  const data = await getJsonOrNull<PnlResponse>(qs ? `/pnl?${qs}` : "/pnl");
  const rows = data?.report?.by_symbol ?? [];
  const total = data?.report?.total;

  return (
    <main className="grid">
      <section className="card">
        <h1>PnL Attribution</h1>
        <p>
          Average-cost realized / unrealized PnL per symbol from a fill stream,
          via the <code>pnl-attribution</code> crate (<code>GET /pnl</code>).
          Drive it with{" "}
          <code>?fills=CAKE,buy,10,2;CAKE,sell,4,3&amp;marks=CAKE:3</code>.
        </p>
        {data?.error ? <p>⚠️ {data.error}</p> : null}
        {total ? (
          <p>
            <strong>realized</strong> {String(total.realized)} ·{" "}
            <strong>unrealized</strong> {String(total.unrealized)} ·{" "}
            <strong>fees</strong> {String(total.fees)} ·{" "}
            <strong>total</strong> {String(total.total)}
          </p>
        ) : null}
        <p>
          <code>fills:</code> {data?.fills ?? "—"}
        </p>
      </section>

      {rows.length > 0 ? (
        <section className="card">
          <h2>By symbol</h2>
          <table>
            <thead>
              <tr>
                <th>symbol</th>
                <th>position</th>
                <th>avg cost</th>
                <th>realized</th>
                <th>unrealized</th>
                <th>fees</th>
                <th>total</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.symbol}>
                  <td>
                    <strong>{r.symbol}</strong>
                  </td>
                  <td>{String(r.position)}</td>
                  <td>{String(r.avg_cost)}</td>
                  <td>{String(r.realized)}</td>
                  <td>{String(r.unrealized)}</td>
                  <td>{String(r.fees)}</td>
                  <td>
                    <strong>{String(r.total)}</strong>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}
    </main>
  );
}
