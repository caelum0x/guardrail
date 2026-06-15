import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface DriftPosition {
  symbol: string;
  status: string;
  current_weight_pct: Numeric;
  target_weight_pct: Numeric;
  delta_pct: Numeric;
  abs_delta_pct: Numeric;
}

interface DriftResponse {
  status: "aligned" | "watch" | "critical" | string;
  regime: string;
  nav_usd: Numeric;
  thresholds: {
    warning_delta_pct: Numeric;
    critical_delta_pct: Numeric;
    max_turnover_pct: Numeric;
  };
  summary: {
    positions: number;
    max_abs_delta_pct: Numeric;
    turnover_pct: Numeric;
    turnover_usd: Numeric;
  };
  positions: DriftPosition[];
  error?: string;
}

function n(value: Numeric, digits = 2): string {
  if (value === null || value === undefined) return "-";
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : String(value);
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
  if (status === "critical") return "critical";
  if (status === "watch") return "warning";
  return "clear";
}

export default async function DriftPage() {
  const data = await getJsonOrNull<DriftResponse>("/drift");
  const positions = Array.isArray(data?.positions) ? data.positions : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "critical")}`}>
        <div>
          <h2>Portfolio Drift</h2>
          {data?.error ? (
            <p>Failed to load drift: {data.error}</p>
          ) : !data ? (
            <p>Drift unavailable.</p>
          ) : (
            <p>Current report weights compared with the latest strategy target.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>Regime</span>
              <strong>{label(data.regime)}</strong>
            </div>
            <div>
              <span>Turnover</span>
              <strong>{pct(data.summary.turnover_pct)}</strong>
            </div>
            <div>
              <span>Turnover USD</span>
              <strong>{usd(data.summary.turnover_usd)}</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Thresholds</h2>
          <div className="metricGrid">
            <div>
              <span>Warning Delta</span>
              <strong>{pct(data.thresholds.warning_delta_pct)}</strong>
            </div>
            <div>
              <span>Critical Delta</span>
              <strong>{pct(data.thresholds.critical_delta_pct)}</strong>
            </div>
            <div>
              <span>Max Turnover</span>
              <strong>{pct(data.thresholds.max_turnover_pct)}</strong>
            </div>
            <div>
              <span>Max Drift</span>
              <strong>{pct(data.summary.max_abs_delta_pct)}</strong>
            </div>
          </div>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Positions</h2>
        {positions.length === 0 ? (
          <p>No drift positions available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Status</th>
                <th>Current</th>
                <th>Target</th>
                <th>Delta</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((position) => (
                <tr key={position.symbol}>
                  <td>{position.symbol}</td>
                  <td>{label(position.status)}</td>
                  <td>{pct(position.current_weight_pct)}</td>
                  <td>{pct(position.target_weight_pct)}</td>
                  <td>{pct(position.delta_pct)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
