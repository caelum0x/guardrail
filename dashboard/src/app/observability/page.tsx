import { getJsonOrNull, getTextOrNull } from "../../lib/api";
import { pctString, usdString } from "../../lib/format";
import type { CockpitResponse } from "../../lib/types";

function metricLine(metrics: string | null, name: string): string {
  if (!metrics) {
    return "0";
  }
  const line = metrics
    .split("\n")
    .find((candidate) => candidate.startsWith(`${name} `));
  return line?.split(/\s+/)[1] ?? "0";
}

export default async function ObservabilityPage() {
  const [cockpit, metrics] = await Promise.all([
    getJsonOrNull<CockpitResponse>("/cockpit"),
    getTextOrNull("/metrics"),
  ]);
  const navUsd =
    cockpit?.run_report?.nav_usd ?? cockpit?.latest_report?.final_nav_usd;
  const totalDrawdownPct =
    cockpit?.run_report?.total_drawdown_pct ??
    cockpit?.latest_report?.total_drawdown_pct;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Observability</h2>
        <div className="metricGrid">
          <div>
            <span>API</span>
            <strong>{cockpit?.health.ok ? "Online" : "Offline"}</strong>
          </div>
          <div>
            <span>NAV</span>
            <strong>{usdString(navUsd)}</strong>
          </div>
          <div>
            <span>Drawdown</span>
            <strong>{pctString(totalDrawdownPct)}</strong>
          </div>
          <div>
            <span>Report age</span>
            <strong>{Number(metricLine(metrics, "guardrail_report_age_seconds")).toFixed(0)}s</strong>
          </div>
          <div>
            <span>Events</span>
            <strong>{metricLine(metrics, "guardrail_events_total")}</strong>
          </div>
          <div>
            <span>Trades</span>
            <strong>{metricLine(metrics, "guardrail_trades_total")}</strong>
          </div>
        </div>
      </section>
      <section className="panel wide">
        <h2>Prometheus Scrape</h2>
        <pre>{metrics ?? "Metrics unavailable. Start guardrail-api and refresh."}</pre>
      </section>
    </main>
  );
}
