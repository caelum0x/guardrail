import Link from "next/link";
import { getJsonOrNull } from "../../lib/api";

interface SkillSummary {
  id: string;
  name: string;
  summary: string;
  regimes?: string[];
  examples_count?: number;
}

interface SkillsResponse {
  count?: number;
  skills?: SkillSummary[];
}

export default async function SkillsPage() {
  const data = await getJsonOrNull<SkillsResponse>("/skills");
  const skills = data?.skills ?? [];

  return (
    <main className="grid">
      <section className="card">
        <h1>Skills Marketplace</h1>
        <p>
          Track-2 strategy skills registered in <code>skills/INDEX.json</code>.
          Each is a backtestable specification; the Rust risk engine remains the
          sole gate on execution.
        </p>
        {skills.length === 0 ? (
          <p>No skills available (is the API running?).</p>
        ) : (
          <ul className="skillList">
            {skills.map((s) => (
              <li key={s.id} className="skillCard">
                <Link href={`/skills/${s.id}`}>
                  <strong>{s.name}</strong>
                </Link>
                <p>{s.summary}</p>
                {s.regimes && s.regimes.length > 0 ? (
                  <small>regimes: {s.regimes.join(", ")}</small>
                ) : null}
                {typeof s.examples_count === "number" ? (
                  <small> · {s.examples_count} examples</small>
                ) : null}
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
