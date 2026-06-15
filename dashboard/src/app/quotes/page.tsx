import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface QuoteRoute {
  route: string;
  side: string;
  amount_usd: Numeric;
  expected_out_symbol: string;
  expected_out_amount: Numeric;
  summary: {
    expected_out_usd: Numeric;
    price_impact_pct: Numeric;
    slippage_pct: Numeric;
    liquidity_usd: Numeric;
  };
  severity: "normal" | "watch" | "high" | string;
}

interface QuotesResponse {
  preview_only: boolean;
  wallet_address: string;
  amount_usd: Numeric;
  routes: QuoteRoute[];
  execution_note: string;
  error?: string;
}

function n(value: Numeric, digits = 2): string {
  if (value === null || value === undefined) {
    return "-";
  }
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return String(value);
  }
  return parsed.toFixed(digits);
}

function pct(value: Numeric): string {
  return `${n(value, 4)}%`;
}

function usd(value: Numeric): string {
  return `$${n(value)}`;
}

function badgeClass(severity: string): string {
  if (severity === "high") {
    return "badge badgeCritical";
  }
  if (severity === "watch") {
    return "badge badgeWarning";
  }
  return "badge";
}

export default async function QuotesPage() {
  const data = await getJsonOrNull<QuotesResponse>("/quotes");
  const routes = Array.isArray(data?.routes) ? data.routes : [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>TWAK Quote Preview</h2>
        {data?.error ? (
          <p>Failed to load quotes: {data.error}</p>
        ) : !data ? (
          <p>Quotes unavailable.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Wallet</span>
              <strong className="mono">{data.wallet_address}</strong>
            </div>
            <div>
              <span>Notional</span>
              <strong>{usd(data.amount_usd)}</strong>
            </div>
            <div>
              <span>Routes</span>
              <strong>{routes.length}</strong>
            </div>
            <div>
              <span>Preview Only</span>
              <strong>{data.preview_only ? "true" : "false"}</strong>
            </div>
          </div>
        )}
      </section>

      <section className="panel wide">
        <h2>Route Quotes</h2>
        {routes.length === 0 ? (
          <p>No route quotes available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Route</th>
                <th>Side</th>
                <th>Expected Out</th>
                <th>Impact</th>
                <th>Slippage</th>
                <th>Liquidity</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {routes.map((route) => (
                <tr key={route.route}>
                  <td>{route.route}</td>
                  <td>{route.side}</td>
                  <td>
                    {n(route.expected_out_amount)} {route.expected_out_symbol}
                  </td>
                  <td>{pct(route.summary.price_impact_pct)}</td>
                  <td>{pct(route.summary.slippage_pct)}</td>
                  <td>{usd(route.summary.liquidity_usd)}</td>
                  <td>
                    <span className={badgeClass(route.severity)}>
                      {route.severity}
                    </span>
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
