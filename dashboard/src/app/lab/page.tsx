import { getJsonOrNull } from "../../lib/api";
import { EquityCurve } from "../../components/EquityCurve";
import { LabControls } from "../../components/LabControls";
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

export default async function LabPage({
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
        <h2>Strategy Lab</h2>
        <p className="eyebrow">
          Tune the strategy inputs and re-run the live backtest interactively.
        </p>
        <LabControls steps={Number(steps)} fearGreed={Number(fearGreed)} />
        <div className="actions">
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
        <EquityCurve points={curve} label="Backtest equity curve" />
      </section>
    </main>
  );
}
