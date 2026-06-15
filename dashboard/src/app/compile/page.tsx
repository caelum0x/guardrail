import { getJsonOrNull } from "../../lib/api";
import { CompileForm } from "../../components/CompileForm";

type CompileResponse = {
  hash?: string;
  policy?: unknown;
  error?: string;
};

export default async function CompilePage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const params = await searchParams;
  const mandate = params.mandate ?? "";
  const trimmed = mandate.trim();

  const result = trimmed
    ? await getJsonOrNull<CompileResponse>(
        `/policy/compile?mandate=${encodeURIComponent(trimmed)}`,
      )
    : null;

  const error = !trimmed
    ? null
    : result === null
      ? "Failed to reach the policy compiler."
      : (result.error ?? null);

  const compiled = result && !result.error ? result : null;

  return (
    <main className="grid">
      <section className="panel wide">
        <p className="eyebrow">Compiler</p>
        <h2>Natural-language policy compiler</h2>
        <p>
          Describe your trading mandate in plain English. The compiler parses it
          into a validated risk policy and returns a deterministic hash that
          binds exactly what the runtime enforces.
        </p>
        <CompileForm initialMandate={mandate} />
      </section>

      {error ? (
        <section className="panel wide">
          <h2>Error</h2>
          <p>{error}</p>
        </section>
      ) : null}

      {compiled ? (
        <section className="panel wide">
          <h2>Compiled policy</h2>
          <div className="metricGrid">
            <div>
              <span>Policy hash</span>
              <strong style={{ fontFamily: "monospace", wordBreak: "break-all" }}>
                {compiled.hash}
              </strong>
            </div>
          </div>
          <pre style={{ overflowX: "auto", whiteSpace: "pre-wrap" }}>
            {JSON.stringify(compiled.policy, null, 2)}
          </pre>
        </section>
      ) : null}
    </main>
  );
}
