import { getJsonOrNull } from "../../lib/api";
import type { JournalCycle, JournalResponse } from "../../lib/types";

/** Title-cases a snake_case/kebab-case regime identifier. */
function humanize(value: string): string {
  return value
    .split(/[_-]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function fmtUsd(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }
  return `$${value.toLocaleString(undefined, { maximumFractionDigits: 2 })}`;
}

function fmtNav(value: string | null | undefined): string {
  if (value === null || value === undefined || value === "") {
    return "—";
  }
  const num = Number(value);
  return Number.isFinite(num) ? fmtUsd(num) : value;
}

/** Compact "risk verdict" summary line for a cycle. */
function riskVerdict(cycle: JournalCycle): string {
  const r = cycle.risk;
  if (!r) {
    return "no risk decisions";
  }
  const parts: string[] = [];
  if (r.approved) parts.push(`${r.approved} approved`);
  if (r.clipped) parts.push(`${r.clipped} clipped`);
  if (r.rejected) parts.push(`${r.rejected} rejected`);
  return parts.length > 0 ? parts.join(" · ") : "no risk decisions";
}

function CycleCard({ cycle }: { cycle: JournalCycle }) {
  const topAssets = cycle.top_assets ?? [];
  const orders = cycle.orders ?? [];
  const reasons = cycle.risk?.rejection_reasons ?? [];

  return (
    <section className="panel">
      <div className="eventRow">
        <h3 style={{ margin: 0 }}>
          Cycle {cycle.index} · {humanize(cycle.regime)}
        </h3>
        <span>
          {cycle.confirmed_trades ?? 0} confirmed · NAV {fmtNav(cycle.ending_nav)}
        </span>
      </div>

      {cycle.headline ? (
        <p style={{ margin: "8px 0" }}>{cycle.headline}</p>
      ) : null}

      <div className="eventRow">
        <span>Regime</span>
        <strong>{humanize(cycle.regime)}</strong>
      </div>
      <div className="eventRow">
        <span>Risk verdict</span>
        <strong>{riskVerdict(cycle)}</strong>
      </div>
      {reasons.length > 0 ? (
        <p className="mono" style={{ color: "#f59e0b" }}>
          Rejections: {reasons.join("; ")}
        </p>
      ) : null}

      {topAssets.length > 0 ? (
        <>
          <p className="eyebrow" style={{ marginTop: 12 }}>
            Top scored assets
          </p>
          <ul className="plainList">
            {topAssets.slice(0, 6).map((asset) => (
              <li key={asset.symbol}>
                <strong>{asset.symbol}</strong> — score{" "}
                {Number.isFinite(asset.score) ? asset.score.toFixed(4) : "—"}
              </li>
            ))}
          </ul>
        </>
      ) : null}

      {orders.length > 0 ? (
        <>
          <p className="eyebrow" style={{ marginTop: 12 }}>
            Proposed orders
          </p>
          <ul className="plainList">
            {orders.map((order, i) => (
              <li key={`${order.from}-${order.to}-${i}`}>
                {order.from} → {order.to}: {fmtUsd(order.amount_usd)}
              </li>
            ))}
          </ul>
        </>
      ) : null}

      <div className="eventRow" style={{ marginTop: 12 }}>
        <span>
          {cycle.started_at ?? "—"} → {cycle.ended_at ?? "—"}
        </span>
        <span>
          {cycle.positions !== null && cycle.positions !== undefined
            ? `${cycle.positions} positions`
            : ""}
          {cycle.run_id ? ` · run ${cycle.run_id}` : ""}
        </span>
      </div>
    </section>
  );
}

export default async function JournalPage() {
  const data = await getJsonOrNull<JournalResponse>("/journal");

  if (!data || data.error) {
    return (
      <main className="grid">
        <section className="panel wide">
          <h2>Decision Journal</h2>
          <p>Journal unavailable{data?.error ? `: ${data.error}` : "."}</p>
        </section>
      </main>
    );
  }

  const cycles = data.cycles ?? [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Decision Journal</h2>
        <p className="eyebrow">
          The verifiable-autonomy narrative reconstructed from the append-only event
          log: regime → scores → orders → risk verdict → confirmed trades.
        </p>
        <div className="metricGrid">
          <div>
            <span>Cycles</span>
            <strong>{data.total_cycles ?? cycles.length}</strong>
          </div>
          <div>
            <span>Events</span>
            <strong>{data.total_events ?? 0}</strong>
          </div>
          <div>
            <span>Confirmed trades</span>
            <strong>{data.confirmed_trades_total ?? 0}</strong>
          </div>
          <div>
            <span>Runs</span>
            <strong>{data.run_ids?.length ?? 0}</strong>
          </div>
        </div>
      </section>

      {cycles.length > 0 ? (
        cycles
          .slice()
          .reverse()
          .map((cycle) => <CycleCard key={cycle.index} cycle={cycle} />)
      ) : (
        <section className="panel wide">
          <p>No decision cycles recorded yet.</p>
        </section>
      )}
    </main>
  );
}
