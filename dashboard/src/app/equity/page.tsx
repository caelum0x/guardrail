import { getJsonOrNull } from "../../lib/api";
import { EquityCurve } from "../../components/EquityCurve";

type EquityPoint = {
  timestamp: string;
  nav_usd: string;
};

type HistoryResponse = {
  points: EquityPoint[];
  count: number;
  error?: string;
};

function formatUsd(value: number): string {
  return value.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export default async function EquityPage() {
  const data = await getJsonOrNull<HistoryResponse>("/history");

  const navValues = (data?.points ?? [])
    .map((point) => Number(point.nav_usd))
    .filter((n) => Number.isFinite(n));

  const hasSeries = navValues.length > 0;
  const first = hasSeries ? navValues[0] : null;
  const last = hasSeries ? navValues[navValues.length - 1] : null;
  const changePct =
    first !== null && last !== null && first !== 0
      ? ((last - first) / first) * 100
      : null;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>NAV Equity Curve</h2>
        <p className="eyebrow">
          Live net asset value over time, reconstructed from PortfolioReconciled events.
        </p>
        {data?.error ? (
          <p className="mono">Error: {data.error}</p>
        ) : !hasSeries ? (
          <p className="mono">No NAV history available yet.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>First NAV</span>
              <strong>${first !== null ? formatUsd(first) : "—"}</strong>
            </div>
            <div>
              <span>Latest NAV</span>
              <strong>${last !== null ? formatUsd(last) : "—"}</strong>
            </div>
            <div>
              <span>Change</span>
              <strong>{changePct !== null ? `${changePct.toFixed(2)}%` : "—"}</strong>
            </div>
            <div>
              <span>Points</span>
              <strong>{data?.count ?? navValues.length}</strong>
            </div>
          </div>
        )}
      </section>
      <section className="panel wide">
        <h2>Equity Curve</h2>
        <EquityCurve points={navValues} label="NAV equity curve" />
      </section>
    </main>
  );
}
