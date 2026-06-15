import { getJsonOrNull } from "../../lib/api";
import { pctString, usdString } from "../../lib/format";

type ExperimentMetrics = {
  total_return_pct?: string;
  max_drawdown_pct?: string;
  trade_count?: number;
  win_rate_pct?: string;
  profit_factor?: string;
  volatility_pct?: string;
  calmar_ratio?: string;
};

type Experiment = {
  tag?: string;
  created_ms?: number;
  steps?: number;
  fear_greed?: number;
  preset?: string;
  metrics?: ExperimentMetrics;
  benchmark_return_pct?: string;
  excess_return_pct?: string;
  final_nav_usd?: string;
};

type ExperimentsResponse = {
  count: number;
  experiments: Experiment[];
};

export default async function ExperimentsPage() {
  const response = await getJsonOrNull<ExperimentsResponse>("/experiments");
  const experiments = response?.experiments ?? [];

  return (
    <main className="grid">
      <section className="panel wide">
        <p className="eyebrow">Experiments</p>
        <h2>Backtest experiment tracking</h2>
        <p>
          Saved backtest experiments are compared side by side so you can see how
          tuning sentiment presets and horizons changes the risk-managed
          trade-off. Each row is one saved run, ordered from oldest to newest.
        </p>
      </section>
      <section className="panel wide">
        {experiments.length === 0 ? (
          <p>
            No experiments saved yet. Run the experiment CLI to record one, for
            example: guardrail experiment --tag my-run --preset balanced
          </p>
        ) : (
          <table>
            <thead>
              <tr>
                <th>Tag</th>
                <th>Preset</th>
                <th>Return %</th>
                <th>Excess %</th>
                <th>Max DD %</th>
                <th>Calmar</th>
                <th>Trades</th>
                <th>Final NAV</th>
              </tr>
            </thead>
            <tbody>
              {experiments.map((experiment, index) => (
                <tr key={experiment.tag ?? `experiment-${index}`}>
                  <td>{experiment.tag ?? "Untitled"}</td>
                  <td>{experiment.preset ?? "—"}</td>
                  <td>{pctString(experiment.metrics?.total_return_pct)}</td>
                  <td>{pctString(experiment.excess_return_pct)}</td>
                  <td>{pctString(experiment.metrics?.max_drawdown_pct)}</td>
                  <td>{experiment.metrics?.calmar_ratio ?? "—"}</td>
                  <td>{experiment.metrics?.trade_count ?? 0}</td>
                  <td>{usdString(experiment.final_nav_usd)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </main>
  );
}
