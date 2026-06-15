import { getJsonOrNull } from "../../lib/api";
import { PresetSelect } from "../../components/PresetSelect";

type WalkForwardWindow = {
  window: number;
  fear_greed: number;
  total_return_pct: string;
  max_drawdown_pct: string;
  benchmark_return_pct: string;
  excess_return_pct: string;
  trades: number;
};

type WalkForwardAggregate = {
  mean_excess_pct: string;
  worst_drawdown_pct: string;
  positive_windows: number;
  total: number;
};

type WalkForwardResponse = {
  windows: WalkForwardWindow[];
  aggregate: WalkForwardAggregate;
  error?: string;
};

export default async function WalkForwardPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const params = await searchParams;
  const windows = params.windows ?? "6";
  const steps = params.steps ?? "30";
  const preset = params.preset ?? "balanced";
  const data = await getJsonOrNull<WalkForwardResponse>(
    `/walkforward?windows=${encodeURIComponent(windows)}&steps=${encodeURIComponent(steps)}&preset=${encodeURIComponent(preset)}`,
  );

  const rows = data?.windows ?? [];
  const agg = data?.aggregate;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Walk-Forward</h2>
        <p className="eyebrow">
          Runs the live strategy across a sequence of windows, each driven by its own fear/greed
          reading, then aggregates per-window performance.
        </p>
        <div className="actions">
          <PresetSelect current={preset} />
        </div>
        {data?.error ? (
          <p className="mono">Error: {data.error}</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Preset</span>
              <strong>{preset}</strong>
            </div>
            <div>
              <span>Windows</span>
              <strong>{agg?.total ?? rows.length}</strong>
            </div>
            <div>
              <span>Positive windows</span>
              <strong>{agg?.positive_windows ?? 0}</strong>
            </div>
            <div>
              <span>Mean excess (alpha)</span>
              <strong>{agg?.mean_excess_pct ?? "—"}%</strong>
            </div>
            <div>
              <span>Worst drawdown</span>
              <strong>{agg?.worst_drawdown_pct ?? "—"}%</strong>
            </div>
          </div>
        )}
      </section>
      <section className="panel wide">
        <h2>Windows</h2>
        {rows.length === 0 ? (
          <p>No windows to display.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Window</th>
                <th>Fear/Greed</th>
                <th>Return %</th>
                <th>Benchmark %</th>
                <th>Excess %</th>
                <th>Max DD %</th>
                <th>Trades</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((w) => (
                <tr key={w.window}>
                  <td>{w.window}</td>
                  <td>{w.fear_greed}</td>
                  <td>{w.total_return_pct}</td>
                  <td>{w.benchmark_return_pct}</td>
                  <td>{w.excess_return_pct}</td>
                  <td>{w.max_drawdown_pct}</td>
                  <td>{w.trades}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
