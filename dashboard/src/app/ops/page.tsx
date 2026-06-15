import { getJsonOrNull } from "../../lib/api";
import type { OpsResponse } from "../../lib/types";

export default async function OpsPage() {
  const ops = await getJsonOrNull<OpsResponse>("/ops");

  return (
    <main className="grid">
      <section className="panel wide">
        <h2>Operations</h2>
        <div className="metricGrid">
          <div>
            <span>API mode</span>
            <strong>{ops?.mode ?? "Pending"}</strong>
          </div>
          <div>
            <span>Docker</span>
            <strong>{ops?.docker.compose ?? "Pending"}</strong>
          </div>
        </div>
      </section>
      <section className="panel wide">
        <h2>Commands</h2>
        <div className="stack">
          {(ops?.operator_commands ?? []).map((item) => (
            <div className="commandRow" key={item.name}>
              <strong>{item.name}</strong>
              <code>{item.command}</code>
            </div>
          ))}
        </div>
      </section>
      <section className="panel">
        <h2>HTTP Surfaces</h2>
        <ul className="plainList">
          {(ops?.http_surfaces ?? []).map((path) => (
            <li key={path}>
              <code>{path}</code>
            </li>
          ))}
        </ul>
      </section>
      <section className="panel">
        <h2>Safety</h2>
        <ul className="plainList">
          {(ops?.safety ?? []).map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </section>
    </main>
  );
}

