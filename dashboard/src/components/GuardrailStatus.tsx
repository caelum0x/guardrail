import type { CockpitResponse } from "../lib/types";
import { pctString, usdString } from "../lib/format";

export function GuardrailStatus({ cockpit }: { cockpit: CockpitResponse | null }) {
  const report = cockpit?.latest_report;
  const online = cockpit?.health.ok ?? false;

  return (
    <section className="panel">
      <h2>Guardrails</h2>
      <div className="metricGrid">
        <div>
          <span>API</span>
          <strong>{online ? "Online" : "Offline"}</strong>
        </div>
        <div>
          <span>NAV</span>
          <strong>{usdString(report?.final_nav_usd)}</strong>
        </div>
        <div>
          <span>Drawdown</span>
          <strong>{pctString(report?.total_drawdown_pct)}</strong>
        </div>
        <div>
          <span>Events</span>
          <strong>{report?.events ?? 0}</strong>
        </div>
      </div>
    </section>
  );
}
