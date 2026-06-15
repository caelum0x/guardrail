import { getJsonOrNull } from "../../lib/api";
import { pctString } from "../../lib/format";
import type { AlertsResponse, GuardrailAlert } from "../../lib/types";

function statusText(alerts: AlertsResponse | null): string {
  if (!alerts) {
    return "API offline";
  }
  if (alerts.status === "critical") {
    return "Critical";
  }
  if (alerts.status === "warning") {
    return "Warning";
  }
  return "Clear";
}

function kindLabel(kind: string): string {
  return kind.replaceAll("_", " ");
}

function actionFor(alert: GuardrailAlert): string {
  if (alert.kind === "kill_switch" || alert.kind === "drawdown_hard") {
    return "Pause execution and inspect risk state.";
  }
  if (alert.kind === "drawdown_soft") {
    return "Review exposure before next rebalance.";
  }
  if (alert.kind === "data_stale") {
    return "Restart the agent or confirm market data access.";
  }
  if (alert.kind === "slippage_high") {
    return "Check TWAK route quality before approving swaps.";
  }
  if (alert.kind === "daily_trade_missing") {
    return "Confirm Track 1 activity before daily cutoff.";
  }
  return "Review the source event and current report.";
}

export default async function AlertsPage() {
  const alerts = await getJsonOrNull<AlertsResponse>("/alerts");
  const rows = alerts?.alerts ?? [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${alerts?.status ?? "critical"}`}>
        <div>
          <p className="eyebrow">Operator alerts</p>
          <h2>{statusText(alerts)}</h2>
        </div>
        <div className="metricGrid">
          <div>
            <span>Critical</span>
            <strong>{alerts?.counts.critical ?? 0}</strong>
          </div>
          <div>
            <span>Warning</span>
            <strong>{alerts?.counts.warning ?? 0}</strong>
          </div>
          <div>
            <span>Total</span>
            <strong>{alerts?.counts.total ?? 0}</strong>
          </div>
          <div>
            <span>Report age</span>
            <strong>{alerts?.inputs.report_age_seconds ?? 0}s</strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <h2>Alert Ledger</h2>
        <div className="alertLedger">
          {rows.length === 0 ? (
            <div className="alertRow clear">
              <strong>No active alerts</strong>
              <span>All evaluated guardrails are inside configured limits.</span>
              <em>Continue monitoring.</em>
            </div>
          ) : (
            rows.map((alert) => (
              <div className={`alertRow ${alert.severity}`} key={`${alert.kind}-${alert.message}`}>
                <strong>{kindLabel(alert.kind)}</strong>
                <span>{alert.message}</span>
                <em>{actionFor(alert)}</em>
              </div>
            ))
          )}
        </div>
      </section>

      <section className="panel wide">
        <h2>Evaluation Inputs</h2>
        <div className="metricGrid">
          <div>
            <span>Drawdown</span>
            <strong>{pctString(alerts?.inputs.total_drawdown_pct)}</strong>
          </div>
          <div>
            <span>Soft limit</span>
            <strong>{pctString(alerts?.inputs.drawdown_soft_limit_pct)}</strong>
          </div>
          <div>
            <span>Hard limit</span>
            <strong>{pctString(alerts?.inputs.drawdown_hard_limit_pct)}</strong>
          </div>
          <div>
            <span>Latest slippage</span>
            <strong>{pctString(alerts?.inputs.latest_slippage_pct)}</strong>
          </div>
          <div>
            <span>Slippage limit</span>
            <strong>{pctString(alerts?.inputs.slippage_limit_pct)}</strong>
          </div>
          <div>
            <span>Kill switch</span>
            <strong>{alerts?.inputs.kill_switch ? "Active" : "Inactive"}</strong>
          </div>
          <div>
            <span>Daily trade</span>
            <strong>{alerts?.inputs.daily_trade_executed ? "Satisfied" : "Missing"}</strong>
          </div>
          <div>
            <span>Trades visible</span>
            <strong>{alerts?.inputs.trades_visible ?? 0}</strong>
          </div>
          <div>
            <span>Events visible</span>
            <strong>{alerts?.inputs.events_visible ?? 0}</strong>
          </div>
          <div>
            <span>Report path</span>
            <strong>{alerts?.inputs.report_path ?? "Pending"}</strong>
          </div>
        </div>
      </section>
    </main>
  );
}
