import { getJsonOrNull } from "../../lib/api";

type Numeric = string | number | null | undefined;

interface HeartbeatResponse {
  name: string;
  status: string;
  requirement: {
    min_trades_per_day: number;
    cooldown_hours: Numeric;
    max_heartbeat_trade_pct: Numeric;
  };
  evidence: {
    recent_confirmed_txs: number;
    daily_marker_present: boolean;
    last_trade_timestamp: string | null;
    last_trade_tx: string | null;
    last_marker_timestamp: string | null;
  };
  plan: {
    needed: boolean;
    from_symbol: string;
    to_symbol: string;
    notional_usd: Numeric;
    nav_usd: Numeric;
    execution_path: string;
    operator_command: string;
  };
  error?: string;
}

function n(value: Numeric, digits = 2): string {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : "-";
}

function statusClass(status: string): string {
  return status === "satisfied" ? "clear" : "warning";
}

function label(value: string): string {
  return value
    .split("_")
    .join(" ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

export default async function HeartbeatPage() {
  const data = await getJsonOrNull<HeartbeatResponse>("/heartbeat");

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "due")}`}>
        <div>
          <h2>Daily Trade Heartbeat</h2>
          {data?.error ? (
            <p>Failed to load heartbeat: {data.error}</p>
          ) : data ? (
            <p>{data.name}</p>
          ) : (
            <p>Heartbeat unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(data.status)}</strong>
            </div>
            <div>
              <span>Confirmed Tx</span>
              <strong>{data.evidence.recent_confirmed_txs}</strong>
            </div>
            <div>
              <span>Daily Marker</span>
              <strong>{data.evidence.daily_marker_present ? "true" : "false"}</strong>
            </div>
            <div>
              <span>Max Heartbeat</span>
              <strong>{n(data.requirement.max_heartbeat_trade_pct)}%</strong>
            </div>
          </div>
        ) : null}
      </section>

      {data ? (
        <section className="panel wide">
          <h2>Plan</h2>
          <div className="metricGrid">
            <div>
              <span>Needed</span>
              <strong>{data.plan.needed ? "true" : "false"}</strong>
            </div>
            <div>
              <span>Pair</span>
              <strong>
                {data.plan.from_symbol} / {data.plan.to_symbol}
              </strong>
            </div>
            <div>
              <span>Notional</span>
              <strong>${n(data.plan.notional_usd)}</strong>
            </div>
            <div>
              <span>NAV</span>
              <strong>${n(data.plan.nav_usd)}</strong>
            </div>
          </div>
        </section>
      ) : null}

      {data ? (
        <section className="panel wide">
          <h2>Evidence</h2>
          <table>
            <thead>
              <tr>
                <th>Last Trade</th>
                <th>Last Tx</th>
                <th>Last Marker</th>
                <th>Execution Path</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="mono">{data.evidence.last_trade_timestamp ?? "-"}</td>
                <td className="mono">{data.evidence.last_trade_tx ?? "-"}</td>
                <td className="mono">{data.evidence.last_marker_timestamp ?? "-"}</td>
                <td>{data.plan.execution_path}</td>
              </tr>
            </tbody>
          </table>
          <p className="mono">{data.plan.operator_command}</p>
        </section>
      ) : null}
    </main>
  );
}
