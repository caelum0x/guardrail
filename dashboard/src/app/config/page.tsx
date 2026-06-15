import { getJsonOrNull } from "../../lib/api";
import type { ConfigResponse } from "../../lib/types";

export default async function ConfigPage() {
  const config = await getJsonOrNull<ConfigResponse>("/config");

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Runtime Configuration</h2>
        <div className="metricGrid">
          <div>
            <span>Database</span>
            <strong className="mono">{config?.environment.database_url ?? "Pending"}</strong>
          </div>
          <div>
            <span>Run report</span>
            <strong className="mono">{config?.environment.report_path ?? "Pending"}</strong>
          </div>
          <div>
            <span>Secrets template</span>
            <strong className="mono">{config?.secrets_template ?? "Pending"}</strong>
          </div>
        </div>
      </section>
      <section className="panel">
        <h2>Paper</h2>
        <pre>{config?.runtime.paper ?? "Pending"}</pre>
      </section>
      <section className="panel">
        <h2>Production</h2>
        <pre>{config?.runtime.production ?? "Pending"}</pre>
      </section>
      <section className="panel">
        <h2>Execution Limits</h2>
        <pre>{JSON.stringify(config?.execution_limits.value ?? {}, null, 2)}</pre>
      </section>
      <section className="panel">
        <h2>Strategy Weights</h2>
        <pre>{JSON.stringify(config?.strategy_weights.value ?? {}, null, 2)}</pre>
      </section>
    </main>
  );
}

