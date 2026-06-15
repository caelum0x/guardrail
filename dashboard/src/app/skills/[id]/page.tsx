import Link from "next/link";
import { getJsonOrNull } from "../../../lib/api";

interface SkillDetail {
  id: string;
  name: string;
  summary?: string;
  description?: string;
  regimes?: string[];
  inputs?: string[];
  examples_count?: number;
  examples_on_disk?: number;
  spec_sections?: string[];
  spec_file?: string;
  error?: string;
}

interface BacktestMetrics {
  total_return_pct?: string;
  max_drawdown_pct?: string;
  trade_count?: number;
  calmar_ratio?: string;
}

interface BacktestResult {
  preset?: string;
  final_nav_usd?: string;
  excess_return_pct?: string;
  metrics?: BacktestMetrics;
  error?: string;
}

export default async function SkillDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const detail = await getJsonOrNull<SkillDetail>(`/skills/${id}`);
  const backtest = await getJsonOrNull<BacktestResult>(
    `/skills/${id}/backtest?preset=balanced`,
  );

  if (!detail || detail.error) {
    return (
      <main className="grid">
        <section className="card">
          <p>
            <Link href="/skills">← Skills</Link>
          </p>
          <h1>Skill not found</h1>
          <p>{detail?.error ?? "API unavailable."}</p>
        </section>
      </main>
    );
  }

  const m = backtest?.metrics;

  return (
    <main className="grid">
      <section className="card">
        <p>
          <Link href="/skills">← Skills</Link>
        </p>
        <h1>{detail.name}</h1>
        <p>{detail.description ?? detail.summary}</p>
        {detail.regimes && detail.regimes.length > 0 ? (
          <p>
            <strong>Regimes:</strong> {detail.regimes.join(", ")}
          </p>
        ) : null}
        {detail.inputs && detail.inputs.length > 0 ? (
          <p>
            <strong>Inputs:</strong> {detail.inputs.join(", ")}
          </p>
        ) : null}
        <p>
          <strong>Examples:</strong> {detail.examples_on_disk ?? detail.examples_count ?? 0}
          {detail.spec_file ? (
            <>
              {" · "}
              <strong>Spec:</strong> <code>{detail.spec_file}</code>
            </>
          ) : null}
        </p>
        {detail.spec_sections && detail.spec_sections.length > 0 ? (
          <p>
            <strong>Spec sections:</strong> {detail.spec_sections.join(", ")}
          </p>
        ) : null}
      </section>

      <section className="card">
        <h2>Backtest (preset: balanced)</h2>
        {!backtest || backtest.error ? (
          <p>Backtest unavailable{backtest?.error ? `: ${backtest.error}` : ""}.</p>
        ) : (
          <ul>
            <li>final NAV: ${backtest.final_nav_usd ?? "—"}</li>
            <li>total return: {m?.total_return_pct ?? "—"}%</li>
            <li>max drawdown: {m?.max_drawdown_pct ?? "—"}%</li>
            <li>excess vs benchmark: {backtest.excess_return_pct ?? "—"}%</li>
            <li>calmar: {m?.calmar_ratio ?? "—"}</li>
            <li>trades: {m?.trade_count ?? "—"}</li>
          </ul>
        )}
      </section>
    </main>
  );
}
