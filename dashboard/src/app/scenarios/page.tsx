import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface ScenarioPosition {
  symbol: string;
  category: string;
  value_usd: Numeric;
  shock_pct: Numeric;
  pnl_usd: Numeric;
  stressed_value_usd: Numeric;
}

interface ScenarioResult {
  id: string;
  label: string;
  description: string;
  status: "normal" | "watch" | "critical" | string;
  portfolio_pnl_usd: Numeric;
  portfolio_return_pct: Numeric;
  largest_loss: {
    symbol: string | null;
    category?: string;
    pnl_usd: Numeric;
    shock_pct?: Numeric;
  };
  positions: ScenarioPosition[];
}

interface ScenariosResponse {
  nav_usd: Numeric;
  worst_scenario_id: string;
  worst_pnl_usd: Numeric;
  scenarios: ScenarioResult[];
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

function statusClass(status: string): string {
  if (status === "critical") {
    return "critical";
  }
  if (status === "watch") {
    return "warning";
  }
  return "clear";
}

export default async function ScenariosPage() {
  const data = await getJsonOrNull<ScenariosResponse>("/scenarios");
  const scenarios = Array.isArray(data?.scenarios) ? data.scenarios : [];
  const worst = scenarios.find((scenario) => scenario.id === data?.worst_scenario_id);

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(worst?.status ?? "critical")}`}>
        <div>
          <h2>Scenario Stress</h2>
          {data?.error ? (
            <p>Failed to load scenarios: {data.error}</p>
          ) : !data ? (
            <p>Scenario stress unavailable.</p>
          ) : (
            <p>Category shocks applied to current report positions.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>NAV</span>
              <strong>{usd(data.nav_usd)}</strong>
            </div>
            <div>
              <span>Worst Scenario</span>
              <strong>{data.worst_scenario_id || "-"}</strong>
            </div>
            <div>
              <span>Worst PnL</span>
              <strong>{usd(data.worst_pnl_usd)}</strong>
            </div>
            <div>
              <span>Scenarios</span>
              <strong>{scenarios.length}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Scenario Results</h2>
        {scenarios.length === 0 ? (
          <p>No scenarios available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Scenario</th>
                <th>Status</th>
                <th>Return</th>
                <th>PnL</th>
                <th>Largest Loss</th>
              </tr>
            </thead>
            <tbody>
              {scenarios.map((scenario) => (
                <tr key={scenario.id}>
                  <td>{scenario.label}</td>
                  <td>{label(scenario.status)}</td>
                  <td>{pct(scenario.portfolio_return_pct)}</td>
                  <td>{usd(scenario.portfolio_pnl_usd)}</td>
                  <td>
                    {scenario.largest_loss.symbol ?? "-"}{" "}
                    {usd(scenario.largest_loss.pnl_usd)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {scenarios.map((scenario) => (
        <section className="panel wide" key={scenario.id}>
          <h2>{scenario.label}</h2>
          <p>{scenario.description}</p>
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Category</th>
                <th>Shock</th>
                <th>PnL</th>
                <th>Stressed Value</th>
              </tr>
            </thead>
            <tbody>
              {scenario.positions.map((position) => (
                <tr key={`${scenario.id}-${position.symbol}`}>
                  <td>{position.symbol}</td>
                  <td>{label(position.category)}</td>
                  <td>{pct(position.shock_pct)}</td>
                  <td>{usd(position.pnl_usd)}</td>
                  <td>{usd(position.stressed_value_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ))}
    </main>
  );
}
