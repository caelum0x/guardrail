import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface ExposurePosition {
  symbol: string;
  category: string;
  value_usd: Numeric;
  weight_pct: Numeric;
}

interface ExposureCategory {
  category: string;
  value_usd: Numeric;
  weight_pct: Numeric;
  positions: number;
}

interface ExposureResponse {
  status: "balanced" | "low_reserve" | "concentrated" | string;
  nav_usd: Numeric;
  report_path: string;
  positions: ExposurePosition[];
  categories: ExposureCategory[];
  summary: {
    position_count: number;
    categorized_positions: number;
    largest_position: ExposurePosition;
    top3_weight_pct: Numeric;
    stable_weight_pct: Numeric;
    risk_weight_pct: Numeric;
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

function pct(value: Numeric): string {
  return `${n(value)}%`;
}

function usd(value: Numeric): string {
  return `$${n(value)}`;
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function statusClass(status: string): string {
  if (status === "concentrated") {
    return "critical";
  }
  if (status === "low_reserve") {
    return "warning";
  }
  return "clear";
}

export default async function ExposurePage() {
  const data = await getJsonOrNull<ExposureResponse>("/exposure");
  const categories = Array.isArray(data?.categories) ? data.categories : [];
  const positions = Array.isArray(data?.positions) ? data.positions : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "critical")}`}>
        <div>
          <h2>Exposure</h2>
          {data?.error ? (
            <p>Failed to load exposure: {data.error}</p>
          ) : !data ? (
            <p>Exposure unavailable.</p>
          ) : (
            <p>
              Current report positions joined to the BSC eligible universe by
              symbol and category.
            </p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>NAV</span>
              <strong>{usd(data.nav_usd)}</strong>
            </div>
            <div>
              <span>Risk Weight</span>
              <strong>{pct(data.summary.risk_weight_pct)}</strong>
            </div>
            <div>
              <span>Stable Weight</span>
              <strong>{pct(data.summary.stable_weight_pct)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Concentration</h2>
          <div className="metricGrid">
            <div>
              <span>Positions</span>
              <strong>{data.summary.position_count}</strong>
            </div>
            <div>
              <span>Categorized</span>
              <strong>{data.summary.categorized_positions}</strong>
            </div>
            <div>
              <span>Top 3 Weight</span>
              <strong>{pct(data.summary.top3_weight_pct)}</strong>
            </div>
            <div>
              <span>Largest Position</span>
              <strong>{data.summary.largest_position.symbol}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Categories</h2>
        {categories.length === 0 ? (
          <p>No category exposure available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Category</th>
                <th>Positions</th>
                <th>Weight</th>
                <th>Value</th>
              </tr>
            </thead>
            <tbody>
              {categories.map((category) => (
                <tr key={category.category}>
                  <td>{label(category.category)}</td>
                  <td>{category.positions}</td>
                  <td>{pct(category.weight_pct)}</td>
                  <td>{usd(category.value_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      <section className="panel wide">
        <h2>Positions</h2>
        {positions.length === 0 ? (
          <p>No positions available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Category</th>
                <th>Weight</th>
                <th>Value</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((position) => (
                <tr key={position.symbol}>
                  <td>{position.symbol}</td>
                  <td>{label(position.category)}</td>
                  <td>{pct(position.weight_pct)}</td>
                  <td>{usd(position.value_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
