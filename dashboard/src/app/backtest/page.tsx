import { getJsonOrNull } from "../../lib/api";
import { PresetSelect } from "../../components/PresetSelect";

type BacktestMetrics = {
  total_return_pct: string;
  max_drawdown_pct: string;
  trade_count: number;
  win_rate_pct: string;
  profit_factor: string;
};

type BacktestResponse = {
  steps: number;
  fear_greed: number;
  starting_nav_usd: string;
  final_nav_usd: string;
  benchmark_return_pct?: string;
  excess_return_pct?: string;
  metrics: BacktestMetrics;
  equity_curve: string[];
  error?: string;
};

const PRESETS: { label: string; fear_greed: number }[] = [
  { label: "Extreme Fear", fear_greed: 15 },
  { label: "Fear", fear_greed: 35 },
  { label: "Neutral", fear_greed: 50 },
  { label: "Greed", fear_greed: 70 },
  { label: "Extreme Greed", fear_greed: 85 },
];

/** Render an equity curve as a self-contained SVG sparkline. */
function Sparkline({ values }: { values: number[] }) {
  if (values.length < 2) {
    return <p>Not enough data to plot.</p>;
  }
  const width = 720;
  const height = 180;
  const pad = 8;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const span = max - min || 1;
  const points = values
    .map((v, i) => {
      const x = pad + (i / (values.length - 1)) * (width - 2 * pad);
      const y = height - pad - ((v - min) / span) * (height - 2 * pad);
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
  const up = values[values.length - 1] >= values[0];
  const stroke = up ? "#22c55e" : "#ef4444";
  return (
    <svg viewBox={`0 0 ${width} ${height}`} width="100%" height={height} role="img" aria-label="Equity curve">
      <polyline points={points} fill="none" stroke={stroke} strokeWidth="2" />
    </svg>
  );
}

export default async function BacktestPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const params = await searchParams;
  const steps = params.steps ?? "60";
  const fearGreed = params.fear_greed ?? "60";
  const preset = params.preset ?? "balanced";
  const data = await getJsonOrNull<BacktestResponse>(
    `/backtest?steps=${encodeURIComponent(steps)}&fear_greed=${encodeURIComponent(fearGreed)}&preset=${encodeURIComponent(preset)}`,
  );

  const curve = (data?.equity_curve ?? []).map((v) => Number(v)).filter((n) => Number.isFinite(n));
  const m = data?.metrics;

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Backtest</h2>
        <p className="eyebrow">
          Runs the live strategy, risk gate, and portfolio accounting over a synthetic price path.
        </p>
        <div className="actions">
          {PRESETS.map((p) => (
            <a
              key={p.fear_greed}
              className="buttonLink"
              href={`/backtest?steps=${steps}&fear_greed=${p.fear_greed}&preset=${preset}`}
            >
              {p.label}
            </a>
          ))}
          <PresetSelect current={preset} />
        </div>
        {data?.error ? (
          <p className="mono">Error: {data.error}</p>
        ) : (
          <div className="metricGrid">
            <div>
              <span>Sentiment (F&amp;G)</span>
              <strong>{data?.fear_greed ?? fearGreed}</strong>
            </div>
            <div>
              <span>Steps</span>
              <strong>{data?.steps ?? steps}</strong>
            </div>
            <div>
              <span>Preset</span>
              <strong>{preset}</strong>
            </div>
            <div>
              <span>Total return</span>
              <strong>{m?.total_return_pct ?? "—"}%</strong>
            </div>
            <div>
              <span>Buy &amp; hold</span>
              <strong>{data?.benchmark_return_pct ?? "—"}%</strong>
            </div>
            <div>
              <span>Excess (alpha)</span>
              <strong>{data?.excess_return_pct ?? "—"}%</strong>
            </div>
            <div>
              <span>Max drawdown</span>
              <strong>{m?.max_drawdown_pct ?? "—"}%</strong>
            </div>
            <div>
              <span>Trades</span>
              <strong>{m?.trade_count ?? 0}</strong>
            </div>
            <div>
              <span>Win rate</span>
              <strong>{m?.win_rate_pct ?? "—"}%</strong>
            </div>
            <div>
              <span>Profit factor</span>
              <strong>{m?.profit_factor ?? "—"}</strong>
            </div>
            <div>
              <span>Final NAV</span>
              <strong>${data?.final_nav_usd ?? "—"}</strong>
            </div>
          </div>
        )}
      </section>
      <section className="panel wide">
        <h2>Equity Curve</h2>
        <Sparkline values={curve} />
      </section>
    </main>
  );
}
