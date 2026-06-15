import { getJsonOrNull } from "../../lib/api";
import type { UniverseResponse, UniverseAsset } from "../../lib/types";

function assets(value: UniverseResponse | null): UniverseAsset[] {
  return Array.isArray(value?.assets) ? value.assets : [];
}

export default async function UniversePage() {
  const universe = await getJsonOrNull<UniverseResponse>("/universe");
  const rows = assets(universe);

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Eligible BSC Universe</h2>
        <div className="metricGrid">
          <div>
            <span>Config path</span>
            <strong className="mono">{universe?.path ?? "Pending"}</strong>
          </div>
          <div>
            <span>Enabled assets</span>
            <strong>{universe?.enabled_assets ?? 0}</strong>
          </div>
        </div>
      </section>
      <section className="panel wide">
        <table>
          <thead>
            <tr>
              <th>Symbol</th>
              <th>Category</th>
              <th>CMC id</th>
              <th>Liquidity floor</th>
              <th>Volume floor</th>
              <th>Contract</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((asset) => (
              <tr key={asset.symbol}>
                <td>{asset.symbol}</td>
                <td>{asset.category}</td>
                <td>{asset.cmc_id}</td>
                <td>${asset.min_liquidity_usd?.toLocaleString() ?? "0"}</td>
                <td>${asset.min_volume_24h_usd?.toLocaleString() ?? "0"}</td>
                <td className="mono">{asset.contract_address}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}

