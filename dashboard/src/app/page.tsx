import { GuardrailStatus } from "../components/GuardrailStatus";
import { MiniSparkline } from "../components/MiniSparkline";
import { PortfolioTable } from "../components/PortfolioTable";
import { RegimeBadge } from "../components/RegimeBadge";
import { RiskPanel } from "../components/RiskPanel";
import { SignalTable } from "../components/SignalTable";
import { TradeTimeline } from "../components/TradeTimeline";
import { getJsonOrNull } from "../lib/api";
import type {
  AlertsResponse,
  CockpitResponse,
  PortfolioResponse,
  RiskResponse,
  SignalsResponse,
} from "../lib/types";

interface CompeteSummary {
  competition_contract: string;
  eligible_assets: number;
  registered: boolean;
  competition_tx: string | null;
  daily_trade_satisfied: boolean;
  confirmed_trades: number;
  kill_switch: boolean;
}

interface HistoryPoint {
  timestamp: string;
  nav_usd: string;
}

interface HistorySummary {
  points: HistoryPoint[];
  count: number;
  error?: string;
}

function formatUsd(value: number): string {
  return value.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function alertStatusText(alerts: AlertsResponse | null): string {
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

export default async function Page() {
  const [cockpit, compete, history, alerts] = await Promise.all([
    getJsonOrNull<CockpitResponse>("/cockpit"),
    getJsonOrNull<CompeteSummary>("/compete"),
    getJsonOrNull<HistorySummary>("/history"),
    getJsonOrNull<AlertsResponse>("/alerts"),
  ]);

  const regime =
    typeof cockpit?.regime?.regime === "string"
      ? cockpit.regime.regime
      : cockpit?.health.ok
        ? "Awaiting regime"
        : "API offline";
  const portfolio: PortfolioResponse = {
    latest: cockpit?.portfolio ?? null,
    source_event_id: null,
  };
  const risk: RiskResponse = {
    kill_switch: cockpit?.risk.kill_switch ?? false,
    events: cockpit?.activity.filter((event) => event.event_type.startsWith("risk_")) ?? [],
  };
  const signals: SignalsResponse = {
    regime: cockpit?.regime ?? null,
    target: cockpit?.target ?? null,
  };

  const navValues = (history?.points ?? [])
    .map((point) => Number(point.nav_usd))
    .filter((value) => Number.isFinite(value));
  const hasNav = navValues.length > 0;
  const latestNav = hasNav ? navValues[navValues.length - 1] : null;
  const firstNav = hasNav ? navValues[0] : null;
  const navChangePct =
    firstNav !== null && latestNav !== null && firstNav !== 0
      ? ((latestNav - firstNav) / firstNav) * 100
      : null;

  const competeReady =
    compete !== null &&
    compete.registered &&
    compete.eligible_assets > 0 &&
    compete.daily_trade_satisfied &&
    compete.confirmed_trades > 0 &&
    !compete.kill_switch;
  const competeStatusClass = compete === null ? "critical" : competeReady ? "clear" : "warning";

  const alertBadgeClass =
    alerts?.status === "critical"
      ? "badge badgeCritical"
      : alerts?.status === "warning"
        ? "badge badgeWarning"
        : "badge";

  return (
    <main className="grid">
      <section className="hero">
        <div>
          <p className="eyebrow">Live cockpit</p>
          <h1>Guardrail Alpha</h1>
        </div>
        <RegimeBadge regime={regime} />
      </section>

      <section className={`panel wide statusPanel ${competeStatusClass}`}>
        <div>
          <p className="eyebrow">Competition readiness</p>
          <h2>{compete === null ? "API offline" : competeReady ? "Ready" : "Action needed"}</h2>
        </div>
        <div className="metricGrid">
          <div>
            <span>Registered</span>
            <strong>{compete?.registered ? "Yes" : "No"}</strong>
          </div>
          <div>
            <span>Eligible assets</span>
            <strong>{compete?.eligible_assets ?? 0}</strong>
          </div>
          <div>
            <span>Daily trade</span>
            <strong>{compete?.daily_trade_satisfied ? "Satisfied" : "Pending"}</strong>
          </div>
          <div>
            <span>Confirmed trades</span>
            <strong>{compete?.confirmed_trades ?? 0}</strong>
          </div>
          <div>
            <span>Kill switch</span>
            <strong>{compete?.kill_switch ? "Engaged" : "Armed"}</strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <div className="hero">
          <div>
            <p className="eyebrow">NAV equity</p>
            <h2>${latestNav !== null ? formatUsd(latestNav) : "—"}</h2>
          </div>
          {navChangePct !== null ? (
            <span className={navChangePct >= 0 ? "badge" : "badge badgeCritical"}>
              {navChangePct >= 0 ? "+" : ""}
              {navChangePct.toFixed(2)}%
            </span>
          ) : null}
        </div>
        {history?.error ? (
          <p className="mono">Error: {history.error}</p>
        ) : hasNav ? (
          <MiniSparkline values={navValues} />
        ) : (
          <p className="mono">No NAV history available yet.</p>
        )}
      </section>

      <section className={`panel wide statusPanel ${alerts?.status ?? "critical"}`}>
        <div className="hero">
          <div>
            <p className="eyebrow">Operator alerts</p>
            <h2>{alertStatusText(alerts)}</h2>
          </div>
          <span className={alertBadgeClass}>{alerts?.counts.total ?? 0} total</span>
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

      <GuardrailStatus cockpit={cockpit} />
      <PortfolioTable portfolio={portfolio} />
      <RiskPanel risk={risk} />
      <SignalTable signals={signals} />
      <TradeTimeline events={cockpit?.activity ?? []} />
    </main>
  );
}
