import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface ExitPosition {
  symbol: string;
  status: string;
  value_usd: Numeric;
  weight_pct: Numeric;
  ret_24h: Numeric;
  synthetic_pnl_pct: Numeric;
  price_usd: Numeric;
  safety_score: number | null;
}

interface ExitResponse {
  thresholds: {
    stop_loss_pct: Numeric;
    take_profit_pct: Numeric;
    warning_loss_pct: Numeric;
    warning_gain_pct: Numeric;
    ret_24h_exit_pct: Numeric;
  };
  summary: {
    positions: number;
    exit: number;
    watch: number;
    hold: number;
  };
  positions: ExitPosition[];
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
  if (status === "exit" || status === "take_profit") return "critical";
  if (status === "watch") return "warning";
  return "clear";
}

export default async function ExitTriggersPage() {
  const data = await getJsonOrNull<ExitResponse>("/exit-triggers");
  const positions = Array.isArray(data?.positions) ? data.positions : [];
  const pageStatus = (data?.summary.exit ?? 0) > 0 ? "exit" : (data?.summary.watch ?? 0) > 0 ? "watch" : "hold";

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(pageStatus)}`}>
        <div>
          <h2>Exit Triggers</h2>
          {data?.error ? (
            <p>Failed to load exit triggers: {data.error}</p>
          ) : !data ? (
            <p>Exit triggers unavailable.</p>
          ) : (
            <p>Current positions evaluated against configured exit thresholds.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Positions</span>
              <strong>{data.summary.positions}</strong>
            </div>
            <div>
              <span>Exit</span>
              <strong>{data.summary.exit}</strong>
            </div>
            <div>
              <span>Watch</span>
              <strong>{data.summary.watch}</strong>
            </div>
            <div>
              <span>Stop Loss</span>
              <strong>{pct(data.thresholds.stop_loss_pct)}</strong>
            </div>
          </div>
        ) : null}
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
                <th>Status</th>
                <th>Weight</th>
                <th>Value</th>
                <th>24h</th>
                <th>Price</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((position) => (
                <tr key={position.symbol}>
                  <td>{position.symbol}</td>
                  <td>{label(position.status)}</td>
                  <td>{pct(position.weight_pct)}</td>
                  <td>{usd(position.value_usd)}</td>
                  <td>{pct(position.ret_24h)}</td>
                  <td>{usd(position.price_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
