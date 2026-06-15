import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface RebalanceTarget {
  symbol: string;
  weight_pct: Numeric;
}

interface RebalanceOrder {
  id: string;
  side: "buy" | "sell" | string;
  from_symbol: string;
  to_symbol: string;
  amount_usd: Numeric;
  reason: string;
}

interface RebalanceDelta {
  symbol: string;
  current_weight_pct: Numeric;
  target_weight_pct: Numeric;
  delta_pct: Numeric;
}

interface RebalanceResponse {
  preview_only: boolean;
  preset: string;
  report_path: string;
  eligible_assets: number;
  nav_usd: Numeric;
  regime: string;
  exposure_multiplier: Numeric;
  thresholds: {
    rebalance_threshold_pct: number;
    max_positions: number;
    max_position_weight_pct: number;
    target_stable_reserve_pct: number;
  };
  summary: {
    target_count: number;
    proposed_orders: number;
    largest_order_usd: Numeric;
    requires_risk_gate: boolean;
  };
  explanation: {
    headline: string;
    top_scores: [string, number][];
  };
  deltas: RebalanceDelta[];
  targets: RebalanceTarget[];
  orders: RebalanceOrder[];
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
  return `$${n(value)}`;
}

function pct(value: Numeric): string {
  return `${n(value)}%`;
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

export default async function RebalancePage() {
  const data = await getJsonOrNull<RebalanceResponse>("/rebalance");
  const orders = Array.isArray(data?.orders) ? data.orders : [];
  const deltas = Array.isArray(data?.deltas) ? data.deltas : [];
  const scores = Array.isArray(data?.explanation?.top_scores)
    ? data.explanation.top_scores
    : [];

  return (
    <main className="grid">
      <section className="panel wide statusPanel">
        <div>
          <h2>Rebalance Preview</h2>
          {data?.error ? (
            <p>Failed to load rebalance preview: {data.error}</p>
          ) : !data ? (
            <p>Rebalance preview unavailable.</p>
          ) : (
            <p>
              {data.explanation.headline} Preview only; proposed orders still
              require the risk gate, TWAK quote, and execution path.
            </p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>NAV</span>
              <strong>{usd(data.nav_usd)}</strong>
            </div>
            <div>
              <span>Regime</span>
              <strong>{label(data.regime)}</strong>
            </div>
            <div>
              <span>Orders</span>
              <strong>{data.summary.proposed_orders}</strong>
            </div>
            <div>
              <span>Largest Order</span>
              <strong>{usd(data.summary.largest_order_usd)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Guardrails</h2>
          <div className="metricGrid">
            <div>
              <span>Preset</span>
              <strong>{data.preset}</strong>
            </div>
            <div>
              <span>Rebalance Threshold</span>
              <strong>{pct(data.thresholds.rebalance_threshold_pct)}</strong>
            </div>
            <div>
              <span>Position Cap</span>
              <strong>{pct(data.thresholds.max_position_weight_pct)}</strong>
            </div>
            <div>
              <span>Reserve Target</span>
              <strong>{pct(data.thresholds.target_stable_reserve_pct)}</strong>
            </div>
          </div>
        </section>
      ) : null}

      {deltas.length > 0 ? (
        <section className="panel wide">
          <h2>Target Deltas</h2>
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Current</th>
                <th>Target</th>
                <th>Delta</th>
              </tr>
            </thead>
            <tbody>
              {deltas.map((row) => (
                <tr key={row.symbol}>
                  <td>{row.symbol}</td>
                  <td>{pct(row.current_weight_pct)}</td>
                  <td>{pct(row.target_weight_pct)}</td>
                  <td>{pct(row.delta_pct)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}

      <section className="panel">
        <h2>Proposed Orders</h2>
        {orders.length === 0 ? (
          <p>No orders proposed.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Side</th>
                <th>Route</th>
                <th>Amount</th>
              </tr>
            </thead>
            <tbody>
              {orders.map((order) => (
                <tr key={order.id}>
                  <td>{label(order.side)}</td>
                  <td>
                    {order.from_symbol} to {order.to_symbol}
                  </td>
                  <td>{usd(order.amount_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel">
        <h2>Top Scores</h2>
        {scores.length === 0 ? (
          <p>No scores available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Score</th>
              </tr>
            </thead>
            <tbody>
              {scores.map(([symbol, score]) => (
                <tr key={symbol}>
                  <td>{symbol}</td>
                  <td>{score.toFixed(3)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
