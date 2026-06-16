import { getJsonOrNull } from "../../lib/api";
import type { CmcCapabilitiesResponse } from "../../lib/types-expansion";

export default async function MarketOraclePage() {
  const data = await getJsonOrNull<CmcCapabilitiesResponse>("/cmc/capabilities");
  const descriptor = data?.descriptor;
  const datasets = descriptor?.datasets ?? [];
  const capabilities = descriptor?.capabilities ?? [];

  return (
    <main className="grid">
      <section className="card">
        <h1>CMC Market Oracle</h1>
        <p>
          The verifiable CoinMarketCap data &rarr; capability lineage served by{" "}
          <code>GET /cmc/capabilities</code>. Each dataset names the exact{" "}
          <code>cmc-client</code> source and the capability it powers. This is a
          read-only analysis surface &mdash; no execution is exposed.
        </p>
        {!data ? (
          <p>Unavailable (is the API running?).</p>
        ) : (
          <p>
            <strong>{data.summary?.cmc_datasets ?? datasets.length}</strong> CMC datasets ·{" "}
            <strong>{data.summary?.exposed_capabilities ?? capabilities.length}</strong>{" "}
            capabilities · execution exposed:{" "}
            <strong>{String(data.summary?.execution_exposed ?? false)}</strong>
          </p>
        )}
      </section>

      {datasets.length > 0 ? (
        <section className="card">
          <h2>CMC datasets</h2>
          <table>
            <thead>
              <tr>
                <th>Dataset</th>
                <th>CMC endpoint</th>
                <th>Powers</th>
                <th>Source</th>
              </tr>
            </thead>
            <tbody>
              {datasets.map((d) => (
                <tr key={d.dataset}>
                  <td>
                    <strong>{d.dataset}</strong>
                  </td>
                  <td>{d.cmc}</td>
                  <td>{d.powers.join(", ")}</td>
                  <td>
                    <code>{d.source}</code>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}

      {capabilities.length > 0 ? (
        <section className="card">
          <h2>Exposed capabilities</h2>
          <table>
            <thead>
              <tr>
                <th>Capability</th>
                <th>CMC inputs</th>
                <th>API</th>
                <th>MCP tool</th>
              </tr>
            </thead>
            <tbody>
              {capabilities.map((c) => (
                <tr key={c.capability}>
                  <td title={c.description}>
                    <strong>{c.capability}</strong>
                  </td>
                  <td>{c.cmc_inputs.join(", ")}</td>
                  <td>{c.api ? <code>{c.api}</code> : "—"}</td>
                  <td>{c.mcp_tool ? <code>{c.mcp_tool}</code> : "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      ) : null}
    </main>
  );
}
