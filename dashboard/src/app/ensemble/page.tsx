import { getJsonOrNull } from "../../lib/api";
import type {
  EnsembleResponse,
  EnsembleSkill,
  EnsembleWeights,
} from "../../lib/types";

/** Title-cases a snake_case or kebab-case regime/skill identifier. */
function humanize(value: string): string {
  return value
    .split(/[_-]/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

/** Formats a 0..1 weight as a percentage string. */
function pct(weight: number): string {
  if (!Number.isFinite(weight)) {
    return "—";
  }
  return `${(weight * 100).toFixed(0)}%`;
}

/** Returns the skill id carrying the largest weight, or null when empty. */
function dominantSkill(weights: EnsembleWeights | null | undefined): string | null {
  if (!weights) {
    return null;
  }
  let leader: string | null = null;
  let best = -Infinity;
  for (const [id, weight] of Object.entries(weights)) {
    if (Number.isFinite(weight) && weight > best) {
      best = weight;
      leader = id;
    }
  }
  return leader;
}

/** Inline SVG-free horizontal bar (pure CSS via inline width). */
function WeightBar({ weight, highlight }: { weight: number; highlight: boolean }) {
  const width = Math.max(0, Math.min(1, weight)) * 100;
  return (
    <div
      style={{
        background: "#1b211f",
        border: "1px solid #2c3433",
        height: 14,
        position: "relative",
        width: "100%",
      }}
      role="img"
      aria-label={`weight ${pct(weight)}`}
    >
      <div
        style={{
          background: highlight ? "#22c55e" : "#3f8f6b",
          height: "100%",
          width: `${width}%`,
        }}
      />
    </div>
  );
}

function WeightTable({
  skills,
  weights,
  dominant,
}: {
  skills: EnsembleSkill[];
  weights: EnsembleWeights;
  dominant: string | null;
}) {
  return (
    <table>
      <thead>
        <tr>
          <th>Skill</th>
          <th>Weight</th>
          <th style={{ width: "45%" }}>Allocation</th>
        </tr>
      </thead>
      <tbody>
        {skills.map((skill) => {
          const weight = weights[skill.id] ?? 0;
          const isDominant = skill.id === dominant;
          return (
            <tr key={skill.id}>
              <td>
                <strong>{humanize(skill.id)}</strong>
                {isDominant ? " ★" : ""}
                <span style={{ color: "#9ead9d", display: "block", fontSize: 12 }}>
                  {skill.label}
                </span>
              </td>
              <td>
                <strong>{pct(weight)}</strong>
              </td>
              <td>
                <WeightBar weight={weight} highlight={isDominant} />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

export default async function EnsemblePage() {
  const data = await getJsonOrNull<EnsembleResponse>("/ensemble");

  if (!data || data.error) {
    return (
      <main className="grid">
        <section className="panel wide">
          <h2>Ensemble</h2>
          <p>Ensemble unavailable{data?.error ? `: ${data.error}` : "."}</p>
        </section>
      </main>
    );
  }

  const skills = data.skills ?? [];
  const currentRegime = data.current_regime ?? null;
  const activeWeights = data.active_weights ?? null;
  const dominant = dominantSkill(activeWeights);

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Regime Ensemble</h2>
        <p className="eyebrow">
          Meta-allocator that blends the strategy Skills by market regime. The active
          row is selected from the current classification.
        </p>
        <div className="metricGrid">
          <div>
            <span>Current regime</span>
            <strong>{currentRegime ? humanize(currentRegime) : "Unknown"}</strong>
          </div>
          <div>
            <span>Dominant skill</span>
            <strong>{dominant ? humanize(dominant) : "—"}</strong>
          </div>
          <div>
            <span>Reserve</span>
            <strong>{data.reserve_symbol ?? "—"}</strong>
          </div>
          <div>
            <span>Max risk alloc</span>
            <strong>
              {data.max_risk_allocation_pct !== undefined
                ? `${data.max_risk_allocation_pct}%`
                : "—"}
            </strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <h2>Active Weights</h2>
        {activeWeights && skills.length > 0 ? (
          <WeightTable skills={skills} weights={activeWeights} dominant={dominant} />
        ) : (
          <p>
            No active blend: the current regime could not be classified. The full
            per-regime table is shown below.
          </p>
        )}
      </section>

      <section className="panel wide">
        <h2>Per-Regime Weight Table</h2>
        {data.regimes && data.regimes.length > 0 ? (
          <div className="stack">
            {data.regimes.map((row) => {
              const rowDominant = dominantSkill(row.weights);
              const isCurrent = row.regime === currentRegime;
              return (
                <div key={row.regime} className="panel">
                  <h3 style={{ margin: "0 0 8px" }}>
                    {humanize(row.regime)}
                    {isCurrent ? (
                      <span className="badge" style={{ marginLeft: 8 }}>
                        current
                      </span>
                    ) : null}
                  </h3>
                  {skills.length > 0 ? (
                    <WeightTable
                      skills={skills}
                      weights={row.weights}
                      dominant={rowDominant}
                    />
                  ) : (
                    <p className="mono">{JSON.stringify(row.weights)}</p>
                  )}
                </div>
              );
            })}
          </div>
        ) : (
          <p>No regime weight table available.</p>
        )}
      </section>
    </main>
  );
}
