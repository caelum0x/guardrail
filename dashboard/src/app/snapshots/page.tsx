import { getJsonOrNull } from "../../lib/api";
import type { SnapshotsResponse } from "../../lib/types";

/** Render a ms-epoch timestamp as a readable UTC string, or a dash. */
function formatTimestamp(ms: number | null | undefined): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) {
    return "—";
  }
  return new Date(ms).toISOString().replace("T", " ").replace(".000Z", " UTC");
}

export default async function SnapshotsPage() {
  const data = await getJsonOrNull<SnapshotsResponse>("/snapshots");

  const runs = data?.runs ?? [];
  const latest = data?.latest ?? null;
  const prices = latest?.latest_prices ?? [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Market Snapshots</h2>
        <p className="eyebrow">
          Read-only history of persisted market snapshots. Summarizes the most
          recent run and a per-asset latest-price sample.
        </p>
        {data === null ? (
          <p className="mono">Snapshot service unavailable.</p>
        ) : !latest ? (
          <p className="mono">No snapshot runs recorded yet.</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Run ID</span>
              <strong className="mono">{latest.run_id}</strong>
            </div>
            <div>
              <span>Cycles</span>
              <strong>{latest.cycle_count}</strong>
            </div>
            <div>
              <span>Skipped lines</span>
              <strong>{latest.skipped_lines}</strong>
            </div>
            <div>
              <span>First snapshot</span>
              <strong>{formatTimestamp(latest.first_timestamp_ms)}</strong>
            </div>
            <div>
              <span>Last snapshot</span>
              <strong>{formatTimestamp(latest.last_timestamp_ms)}</strong>
            </div>
            <div>
              <span>Discovered runs</span>
              <strong>{runs.length}</strong>
            </div>
          </div>
        )}
      </section>

      <section className="panel wide">
        <h2>Latest Price Sample</h2>
        {prices.length === 0 ? (
          <p className="mono">No per-asset prices in the latest snapshot.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Price (USD)</th>
              </tr>
            </thead>
            <tbody>
              {prices.map((sample) => (
                <tr key={sample.symbol}>
                  <td>{sample.symbol}</td>
                  <td className="mono">{sample.price_usd}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {runs.length > 0 ? (
        <section className="panel wide">
          <h2>Discovered Runs</h2>
          <table>
            <thead>
              <tr>
                <th>Run ID</th>
                <th>Last modified</th>
              </tr>
            </thead>
            <tbody>
              {runs.map((run) => (
                <tr key={run.run_id}>
                  <td className="mono">{run.run_id}</td>
                  <td>{formatTimestamp(run.modified_ms)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}
    </main>
  );
}
