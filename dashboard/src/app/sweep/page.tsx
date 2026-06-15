import { getJsonOrNull } from "../../lib/api";
import { PresetSelect } from "../../components/PresetSelect";

type SweepRow = {
  fear_greed: number;
  total_return_pct: string;
  benchmark_return_pct: string;
  excess_return_pct: string;
  max_drawdown_pct: string;
  trade_count: number;
};

type SweepResponse = {
  steps: number;
  rows: SweepRow[];
  error?: string;
};

export default async function SweepPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const params = await searchParams;
  const steps = params.steps ?? "40";
  const fearGreed = params.fear_greed ?? "20,40,60,80";
  const preset = params.preset ?? "balanced";
  const data = await getJsonOrNull<SweepResponse>(
    `/sweep?steps=${encodeURIComponent(steps)}&fear_greed=${encodeURIComponent(fearGreed)}&preset=${encodeURIComponent(preset)}`,
  );

  const rows = data?.rows ?? [];

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Sentiment Sweep</h2>
        <p className="eyebrow">
          Runs the live strategy, risk gate, and portfolio accounting once per fear/greed reading
          and compares it against buy &amp; hold over the same synthetic price path.
        </p>
        <p>
          The strategy preserves capital in fear (smaller drawdowns, positive excess vs. buy &amp;
          hold) and deliberately lags an all-in book in euphoria — it trades upside in greed for
          downside protection in fear.
        </p>
        <div className="actions">
          <PresetSelect current={preset} />
        </div>
        <p className="eyebrow">Active preset: {preset}</p>
        {data?.error ? (
          <p className="mono">Error: {data.error}</p>
        ) : rows.length === 0 ? (
          <p className="mono">No data available.</p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Fear &amp; Greed</th>
                <th>Strategy return %</th>
                <th>Buy &amp; hold %</th>
                <th>Excess %</th>
                <th>Max drawdown %</th>
                <th>Trades</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.fear_greed}>
                  <td>{row.fear_greed}</td>
                  <td>{row.total_return_pct}</td>
                  <td>{row.benchmark_return_pct}</td>
                  <td>{row.excess_return_pct}</td>
                  <td>{row.max_drawdown_pct}</td>
                  <td>{row.trade_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
