import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface BudgetResponse {
  status: "funded" | "watch" | "blocking" | string;
  budget: {
    name: string;
    daily_trade_target: Numeric;
    planned_competition_days: Numeric;
    operator_gas_float_usd: Numeric;
    max_daily_execution_cost_usd: Numeric;
    max_cost_bps_per_trade: Numeric;
    min_nav_usd: Numeric;
  };
  current: {
    nav_usd: Numeric;
    default_order_notional_usd: Numeric;
    gas_usd_per_trade: Numeric;
    slippage_usd_per_trade: Numeric;
    cost_usd_per_trade: Numeric;
    cost_bps_per_trade: Numeric;
    daily_execution_cost_usd: Numeric;
    planned_execution_cost_usd: Numeric;
    runway_days: Numeric;
  };
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

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusClass(status: string): string {
  if (status === "blocking") {
    return "critical";
  }
  if (status === "watch") {
    return "warning";
  }
  return "clear";
}

export default async function BudgetPage() {
  const data = await getJsonOrNull<BudgetResponse>("/budget");

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "blocking")}`}>
        <div>
          <h2>Trading Budget</h2>
          {data?.error ? (
            <p>Failed to load budget: {data.error}</p>
          ) : !data ? (
            <p>Budget unavailable.</p>
          ) : (
            <p>{data.budget.name}</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>Runway Days</span>
              <strong>{n(data.current.runway_days)}</strong>
            </div>
            <div>
              <span>Daily Cost</span>
              <strong>{usd(data.current.daily_execution_cost_usd)}</strong>
            </div>
            <div>
              <span>Cost BPS</span>
              <strong>{n(data.current.cost_bps_per_trade)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Limits</h2>
          <div className="metricGrid">
            <div>
              <span>Gas Float</span>
              <strong>{usd(data.budget.operator_gas_float_usd)}</strong>
            </div>
            <div>
              <span>Max Daily Cost</span>
              <strong>{usd(data.budget.max_daily_execution_cost_usd)}</strong>
            </div>
            <div>
              <span>Max Cost BPS</span>
              <strong>{n(data.budget.max_cost_bps_per_trade)}</strong>
            </div>
            <div>
              <span>Min NAV</span>
              <strong>{usd(data.budget.min_nav_usd)}</strong>
            </div>
          </div>
        </section>
      ) : null}

      {data ? (
        <section className="panel wide">
          <h2>Current Estimate</h2>
          <table>
            <thead>
              <tr>
                <th>Notional</th>
                <th>Gas</th>
                <th>Slippage</th>
                <th>Per Trade</th>
                <th>Planned Cost</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>{usd(data.current.default_order_notional_usd)}</td>
                <td>{usd(data.current.gas_usd_per_trade)}</td>
                <td>{usd(data.current.slippage_usd_per_trade)}</td>
                <td>{usd(data.current.cost_usd_per_trade)}</td>
                <td>{usd(data.current.planned_execution_cost_usd)}</td>
              </tr>
            </tbody>
          </table>
        </section>
      ) : null}
    </main>
  );
}
