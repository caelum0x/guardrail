import { getJsonOrNull } from "../../lib/api";

interface CompeteResponse {
  competition_contract: string;
  competition_contract_bsctrace: string;
  eligible_assets: number;
  registered: boolean;
  competition_tx: string | null;
  daily_trade_satisfied: boolean;
  confirmed_trades: number;
  kill_switch: boolean;
}

interface ChecklistRow {
  label: string;
  pass: boolean;
  detail: string;
}

function buildChecklist(compete: CompeteResponse): ChecklistRow[] {
  return [
    {
      label: "Registered with competition contract",
      pass: compete.registered,
      detail: compete.competition_tx
        ? `tx ${compete.competition_tx}`
        : "no competition_tx observed",
    },
    {
      label: "Eligible assets",
      pass: compete.eligible_assets > 0,
      detail: `${compete.eligible_assets} enabled`,
    },
    {
      label: "Daily-trade requirement satisfied",
      pass: compete.daily_trade_satisfied,
      detail: compete.daily_trade_satisfied ? "satisfied" : "pending",
    },
    {
      label: "Confirmed trades",
      pass: compete.confirmed_trades > 0,
      detail: `${compete.confirmed_trades} confirmed`,
    },
    {
      label: "Kill switch",
      pass: !compete.kill_switch,
      detail: compete.kill_switch ? "engaged" : "armed",
    },
  ];
}

export default async function CompetePage() {
  const compete = await getJsonOrNull<CompeteResponse>("/compete");

  if (!compete) {
    return (
      <main className="grid">
        <section className="panel wide statusPanel critical">
          <div>
            <p className="eyebrow">Track-1 readiness</p>
            <h2>API offline</h2>
          </div>
          <p>Unable to load /compete. Start the API and retry.</p>
        </section>
      </main>
    );
  }

  const checklist = buildChecklist(compete);
  const allReady = checklist.every((row) => row.pass);

  return (
    <main className="grid">
      <section className={`panel wide statusPanel ${allReady ? "clear" : "critical"}`}>
        <div>
          <p className="eyebrow">Track-1 readiness</p>
          <h2>{allReady ? "Ready" : "Action needed"}</h2>
        </div>
        <div className="metricGrid">
          <div>
            <span>Registered</span>
            <strong>{compete.registered ? "Yes" : "No"}</strong>
          </div>
          <div>
            <span>Eligible assets</span>
            <strong>{compete.eligible_assets}</strong>
          </div>
          <div>
            <span>Confirmed trades</span>
            <strong>{compete.confirmed_trades}</strong>
          </div>
          <div>
            <span>Kill switch</span>
            <strong>{compete.kill_switch ? "Engaged" : "Armed"}</strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <h2>Readiness checklist</h2>
        <div className="alertLedger">
          {checklist.map((row) => (
            <div className={`alertRow ${row.pass ? "clear" : "critical"}`} key={row.label}>
              <strong>
                {row.pass ? "✓" : "✗"} {row.label}
              </strong>
              <span>{row.detail}</span>
              <em>{row.pass ? "OK" : "Pending"}</em>
            </div>
          ))}
        </div>
      </section>

      <section className="panel wide">
        <h2>Competition contract</h2>
        <div className="metricGrid">
          <div>
            <span>Contract</span>
            <strong>
              <a
                className="mono link"
                href={compete.competition_contract_bsctrace}
              >
                {compete.competition_contract}
              </a>
            </strong>
          </div>
          <div>
            <span>Explorer</span>
            <strong>
              <a className="mono link" href={compete.competition_contract_bsctrace}>
                BSCTrace
              </a>
            </strong>
          </div>
          {compete.competition_tx ? (
            <div>
              <span>Registration tx</span>
              <strong>
                <a
                  className="mono link"
                  href={`https://bscscan.com/tx/${compete.competition_tx}`}
                >
                  {compete.competition_tx}
                </a>
              </strong>
            </div>
          ) : null}
        </div>
      </section>
    </main>
  );
}
