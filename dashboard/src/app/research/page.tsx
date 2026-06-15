import { getJsonOrNull } from "../../lib/api";
import { pctString } from "../../lib/format";

type BacktestMetrics = {
  total_return_pct: string;
  max_drawdown_pct: string;
};

type BacktestResponse = {
  fear_greed: number;
  final_nav_usd: string;
  excess_return_pct?: string;
  metrics: BacktestMetrics;
  error?: string;
};

type ResearchTool = {
  href: string;
  title: string;
  description: string;
};

const TOOLS: ResearchTool[] = [
  {
    href: "/backtest",
    title: "Backtest",
    description:
      "Runs a single backtest across sentiment presets to show how the strategy behaves from extreme fear to extreme greed.",
  },
  {
    href: "/lab",
    title: "Lab",
    description:
      "Interactive backtest controls let you tune sentiment and horizon inputs and watch the equity curve respond in real time.",
  },
  {
    href: "/walkforward",
    title: "Walk-forward",
    description:
      "Evaluates the strategy over rolling out-of-sample windows to confirm performance is not a single-period fluke.",
  },
  {
    href: "/sweep",
    title: "Sweep",
    description:
      "Compares outcomes across the full sentiment spectrum side by side to quantify the risk-managed trade-off.",
  },
];

export default async function ResearchPage() {
  const backtest = await getJsonOrNull<BacktestResponse>("/backtest");
  const headline = backtest && !backtest.error ? backtest : null;

  return (
    <main className="grid">
      <section className="panel wide">
        <p className="eyebrow">Research</p>
        <h2>Research hub</h2>
        <p>
          The strategy is risk-managed by design: it preserves capital when the
          market is fearful and intentionally lags an all-in benchmark during
          euphoria. The tools below quantify that trade-off so you can see
          exactly where the guardrails help and where they cost.
        </p>
        {headline ? (
          <div className="metricGrid">
            <div>
              <span>Sample fear/greed</span>
              <strong>{headline.fear_greed}</strong>
            </div>
            <div>
              <span>Total return</span>
              <strong>{pctString(headline.metrics.total_return_pct)}</strong>
            </div>
            <div>
              <span>Max drawdown</span>
              <strong>{pctString(headline.metrics.max_drawdown_pct)}</strong>
            </div>
            <div>
              <span>Excess vs benchmark</span>
              <strong>{pctString(headline.excess_return_pct)}</strong>
            </div>
          </div>
        ) : null}
      </section>
      {TOOLS.map((tool) => (
        <section key={tool.href} className="panel">
          <h2>{tool.title}</h2>
          <p>{tool.description}</p>
          <div className="actions">
            <a className="buttonLink" href={tool.href}>
              Open {tool.title}
            </a>
          </div>
        </section>
      ))}
    </main>
  );
}
