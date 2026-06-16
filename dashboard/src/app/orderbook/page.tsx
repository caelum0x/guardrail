import { getJsonOrNull } from "../../lib/api";

interface Level {
  price: string;
  quantity: string;
}
interface Trade {
  taker_id: number;
  maker_id: number;
  price: string;
  quantity: string;
}
interface OrderbookResponse {
  spec?: string;
  trades?: Trade[];
  trade_count?: number;
  best_bid?: string | null;
  best_ask?: string | null;
  spread?: string | null;
  depth?: { bids?: Level[]; asks?: Level[] };
  resting_orders?: number;
  error?: string;
}

export default async function OrderbookPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const p = await searchParams;
  const orders = p.orders;
  const qs = orders ? `?orders=${encodeURIComponent(orders)}` : "";
  const data = await getJsonOrNull<OrderbookResponse>(`/orderbook${qs}`);

  return (
    <main className="grid">
      <section className="card">
        <h1>Order Book — matching engine</h1>
        <p>
          Runs the real <code>orderbook</code> crate&apos;s price-time-priority
          matching engine over a compact order spec
          (<code>GET /orderbook</code>). Drive it with{" "}
          <code>?orders=b,limit,100,5;s,market,,3</code>.
        </p>
        {data?.error ? <p>⚠️ {data.error}</p> : null}
        {data && !data.error ? (
          <p>
            <strong>best bid</strong> {data.best_bid ?? "—"} ·{" "}
            <strong>best ask</strong> {data.best_ask ?? "—"} ·{" "}
            <strong>spread</strong> {data.spread ?? "—"} ·{" "}
            <strong>{data.trade_count ?? 0}</strong> trades ·{" "}
            <strong>{data.resting_orders ?? 0}</strong> resting
          </p>
        ) : null}
        <p>
          <code>spec:</code> {data?.spec ?? "—"}
        </p>
      </section>

      {data?.trades && data.trades.length > 0 ? (
        <section className="card">
          <h2>Trades</h2>
          <table>
            <thead>
              <tr>
                <th>price</th>
                <th>qty</th>
                <th>taker</th>
                <th>maker</th>
              </tr>
            </thead>
            <tbody>
              {data.trades.map((t, i) => (
                <tr key={i}>
                  <td>{t.price}</td>
                  <td>{t.quantity}</td>
                  <td>{t.taker_id}</td>
                  <td>{t.maker_id}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}

      <section className="card">
        <h2>Depth</h2>
        <div className="grid">
          <div>
            <h3>Bids</h3>
            <table>
              <tbody>
                {(data?.depth?.bids ?? []).map((l, i) => (
                  <tr key={i}>
                    <td>{l.price}</td>
                    <td>{l.quantity}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div>
            <h3>Asks</h3>
            <table>
              <tbody>
                {(data?.depth?.asks ?? []).map((l, i) => (
                  <tr key={i}>
                    <td>{l.price}</td>
                    <td>{l.quantity}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>
    </main>
  );
}
