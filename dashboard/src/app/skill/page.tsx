import { getJsonOrNull } from "../../lib/api";

interface SkillResponse {
  name: string;
  skill_yaml: string;
  readme: string;
  examples: string[];
}

function examples(value: SkillResponse | null): string[] {
  return Array.isArray(value?.examples) ? value.examples : [];
}

export default async function SkillPage() {
  const data = await getJsonOrNull<SkillResponse>("/skill");
  const exampleFiles = examples(data);
  const readme = data?.readme ?? "";
  const skillYaml = data?.skill_yaml ?? "";

  return (
    <main className="grid">
      <section className="panel wide">
        <p className="eyebrow">CMC Skill</p>
        <h2>{data?.name ?? "cmc-regime-routed-alpha"}</h2>
        <p>
          The Track 2 CMC Skill artifact: a regime-routed BSC trading strategy
          packaged from CMC quotes, OHLCV, DEX liquidity, sentiment, Fear &amp;
          Greed, and trending data.
        </p>
        {!data ? (
          <p className="mono">Error: Unable to load the CMC Skill artifact.</p>
        ) : null}
      </section>

      {readme ? (
        <section className="panel wide">
          <h2>README</h2>
          <pre className="mono">{readme}</pre>
        </section>
      ) : null}

      {skillYaml ? (
        <section className="panel wide">
          <h2>skill.yaml</h2>
          <pre className="mono">{skillYaml}</pre>
        </section>
      ) : null}

      <section className="panel wide">
        <h2>Examples</h2>
        {exampleFiles.length === 0 ? (
          <p>No example files available.</p>
        ) : (
          <ul>
            {exampleFiles.map((file) => (
              <li key={file} className="mono">
                {file}
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}
