import { getJsonOrNull } from "../../lib/api";

interface ModuleRow {
  name: string;
  path: string;
  present: boolean;
  files: number;
}

interface ExampleRow {
  name: string;
  path: string;
  files: number;
}

interface FileRow {
  name: string;
  path: string;
  present: boolean;
  bytes: number;
}

interface SdkCatalogResponse {
  root: string;
  status: string;
  summary: {
    files: number;
    modules: number;
    modules_present: number;
    examples: number;
    tests: number;
    abis: number;
  };
  modules: ModuleRow[];
  examples: ExampleRow[];
  top_files: FileRow[];
  error?: string;
}

function statusClass(status: string): string {
  return status === "present" ? "clear" : "critical";
}

export default async function SdkCatalogPage() {
  const data = await getJsonOrNull<SdkCatalogResponse>("/sdk-catalog");
  const modules = Array.isArray(data?.modules) ? data.modules : [];
  const examples = Array.isArray(data?.examples) ? data.examples : [];
  const files = Array.isArray(data?.top_files) ? data.top_files : [];

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${statusClass(data?.status ?? "missing")}`}>
        <div>
          <h2>BNB SDK Catalog</h2>
          {data?.error ? (
            <p>Failed to load SDK catalog: {data.error}</p>
          ) : data ? (
            <p>{data.root}</p>
          ) : (
            <p>SDK catalog unavailable.</p>
          )}
        </div>
        {data ? (
          <div className="metricGrid">
            <div>
              <span>Files</span>
              <strong>{data.summary.files}</strong>
            </div>
            <div>
              <span>Modules</span>
              <strong>
                {data.summary.modules_present}/{data.summary.modules}
              </strong>
            </div>
            <div>
              <span>Examples</span>
              <strong>{data.summary.examples}</strong>
            </div>
            <div>
              <span>ABI Files</span>
              <strong>{data.summary.abis}</strong>
            </div>
          </div>
        ) : null}
      </section>

      <section className="panel wide">
        <h2>Modules</h2>
        <table>
          <thead>
            <tr>
              <th>Module</th>
              <th>Present</th>
              <th>Files</th>
              <th>Path</th>
            </tr>
          </thead>
          <tbody>
            {modules.map((module) => (
              <tr key={module.name}>
                <td>{module.name}</td>
                <td>{module.present ? "true" : "false"}</td>
                <td>{module.files}</td>
                <td className="mono">{module.path}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section className="panel wide">
        <h2>Examples</h2>
        <div className="metricGrid">
          {examples.map((example) => (
            <div key={example.name}>
              <span>{example.name}</span>
              <strong>{example.files}</strong>
            </div>
          ))}
        </div>
      </section>

      <section className="panel wide">
        <h2>Top Files</h2>
        <table>
          <thead>
            <tr>
              <th>File</th>
              <th>Status</th>
              <th>Bytes</th>
              <th>Path</th>
            </tr>
          </thead>
          <tbody>
            {files.map((file) => (
              <tr key={file.name}>
                <td>{file.name}</td>
                <td>{file.present ? "present" : "missing"}</td>
                <td>{file.bytes}</td>
                <td className="mono">{file.path}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </main>
  );
}
