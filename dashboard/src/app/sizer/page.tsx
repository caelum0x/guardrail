import { getJsonOrNull } from "../../lib/api";

interface SizerResponse {
  method?: string;
  input?: Record<string, unknown>;
  output?: Record<string, unknown>;
  error?: string;
  methods?: string[];
}

const METHODS = ["fixed_fractional", "vol_target", "kelly"] as const;

function rows(obj: Record<string, unknown> | undefined): Array<[string, string]> {
  if (!obj) return [];
  return Object.entries(obj).map(([k, v]) => [k, String(v)]);
}

export default async function SizerPage({
  searchParams,
}: {
  searchParams: Promise<Record<string, string | undefined>>;
}) {
  const p = await searchParams;
  const method = METHODS.includes((p.method ?? "") as (typeof METHODS)[number])
    ? (p.method as string)
    : "kelly";
  // Pass through any provided numeric params verbatim.
  const qs = new URLSearchParams({ method });
  for (const [k, v] of Object.entries(p)) {
    if (v !== undefined && k !== "method") qs.set(k, v);
  }
  const data = await getJsonOrNull<SizerResponse>(`/sizer?${qs.toString()}`);

  return (
    <main className="grid">
      <section className="card">
        <h1>Position Sizer</h1>
        <p>
          Position sizing via the <code>position-sizer</code> crate
          (<code>GET /sizer</code>). Choose <code>?method=</code>{" "}
          {METHODS.map((m) => (
            <code key={m} style={{ marginRight: 6 }}>
              {m}
            </code>
          ))}
          and pass that method&apos;s params.
        </p>
        <p>
          <strong>method:</strong> {method}
        </p>
        {data?.error ? <p>⚠️ {data.error}</p> : null}
      </section>

      {data?.input ? (
        <section className="card">
          <h2>Input</h2>
          <table>
            <tbody>
              {rows(data.input).map(([k, v]) => (
                <tr key={k}>
                  <td>{k}</td>
                  <td>{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}

      {data?.output ? (
        <section className="card">
          <h2>Output</h2>
          <table>
            <tbody>
              {rows(data.output).map(([k, v]) => (
                <tr key={k}>
                  <td>{k}</td>
                  <td>
                    <strong>{v}</strong>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}
    </main>
  );
}
