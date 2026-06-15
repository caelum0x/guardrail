import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface CostRoute {
  route: string;
  side: string;
  amount_usd: Numeric;
  gas_units: Numeric;
  gas_usd: Numeric;
  slippage_usd: Numeric;
  all_in_cost_usd: Numeric;
  all_in_cost_bps: Numeric;
  price_impact_pct: Numeric;
  slippage_pct: Numeric;
}

interface CostsResponse {
  preview_only: boolean;
  chain: string;
  native_symbol: string;
  summary: {
    routes: number;
    amount_usd: Numeric;
    total_gas_usd: Numeric;
    total_slippage_usd: Numeric;
    total_all_in_cost_usd: Numeric;
    average_cost_bps: Numeric;
  };
  routes: CostRoute[];
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

function usd(value: Numeric): string {
  return `$${n(value, 4)}`;
}

function pct(value: Numeric): string {
  return `${n(value, 4)}%`;
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export default async function CostsPage() {
  const data = await getJsonOrNull<CostsResponse>("/costs");
  const routes = Array.isArray(data?.routes) ? data.routes : [];

  return (
    <main className="grid">
      <section className="panel wide statusPanel clear">
        <div>
          <h2>Execution Costs</h2>
          {data?.error ? (
            <p>Failed to load costs: {data.error}</p>
          ) : !data ? (
            <p>Execution costs unavailable.</p>
          ) : (
            <p>Gas and slippage estimates for configured TWAK preview routes.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Chain</span>
              <strong>{String(data.chain).toUpperCase()}</strong>
            </div>
            <div>
              <span>Routes</span>
              <strong>{data.summary.routes}</strong>
            </div>
            <div>
              <span>Total Cost</span>
              <strong>{usd(data.summary.total_all_in_cost_usd)}</strong>
            </div>
            <div>
              <span>Average BPS</span>
              <strong>{n(data.summary.average_cost_bps)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Routes</h2>
        {routes.length === 0 ? (
          <p>No route costs available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Route</th>
                <th>Side</th>
                <th>Gas</th>
                <th>Slippage</th>
                <th>All-In</th>
                <th>BPS</th>
                <th>Impact</th>
              </tr>
            </thead>
            <tbody>
              {routes.map((route) => (
                <tr key={route.route}>
                  <td>{route.route}</td>
                  <td>{label(route.side)}</td>
                  <td>{usd(route.gas_usd)}</td>
                  <td>{usd(route.slippage_usd)}</td>
                  <td>{usd(route.all_in_cost_usd)}</td>
                  <td>{n(route.all_in_cost_bps)}</td>
                  <td>{pct(route.price_impact_pct)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
