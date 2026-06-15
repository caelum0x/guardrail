import { API_URL, getJsonOrNull } from "../../lib/api";
import type {
  CompeteResponse,
  PrizesResponse,
  ProofVerifyResponse,
  ReadinessResponse,
  ScorecardResponse,
} from "../../lib/types";

function statusClass(status: string): string {
  return status === "ready" ? "clear" : status === "blocking" ? "critical" : "warning";
}

function label(value: string): string {
  return value
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function CompeteChip({ ok, text }: { ok: boolean; text: string }) {
  return (
    <span className={`badge ${ok ? "" : "badgeCritical"}`} style={{ marginRight: 8 }}>
      {ok ? "✓" : "✗"} {text}
    </span>
  );
}

export default async function SubmissionPage() {
  const [readiness, verify, scorecard, prizes, compete] = await Promise.all([
    getJsonOrNull<ReadinessResponse>("/readiness"),
    getJsonOrNull<ProofVerifyResponse>("/proof/verify"),
    getJsonOrNull<ScorecardResponse>("/scorecard"),
    getJsonOrNull<PrizesResponse>("/prizes"),
    getJsonOrNull<CompeteResponse>("/compete"),
  ]);

  const readinessChecks = readiness?.checks ?? [];
  const isReady = readiness?.status === "ready";
  const blocking = readiness?.blocking ?? readinessChecks.length;
  const verifyChecks = verify?.checks ?? [];
  const scoreSections = Array.isArray(scorecard?.sections) ? scorecard.sections : [];
  const prizeRows = Array.isArray(prizes?.prizes) ? prizes.prizes : [];

  return (
    <main className="grid">
      <section
        className={`panel wide statusPanel ${
          readiness ? (isReady ? "clear" : "critical") : "warning"
        }`}
      >
        <div>
          <p className="eyebrow">Judge submission</p>
          <h2>
            {!readiness
              ? "Readiness unavailable"
              : isReady
                ? "READY"
                : "NOT READY"}
          </h2>
        </div>
        <div className="metricGrid">
          <div>
            <span>Checks</span>
            <strong>{readinessChecks.length}</strong>
          </div>
          <div>
            <span>Blocking</span>
            <strong>{blocking}</strong>
          </div>
          <div>
            <span>Proof verify</span>
            <strong>
              {!verify ? "n/a" : verify.passed ? "Passed" : "Failed"}
            </strong>
          </div>
          <div>
            <span>Scorecard</span>
            <strong>
              {scorecard ? `${scorecard.summary.score_pct}%` : "n/a"}
            </strong>
          </div>
        </div>
      </section>

      <section className="panel wide">
        <h2>Readiness checklist</h2>
        {!readiness ? (
          <p>Unable to load /readiness. Start the API and retry.</p>
        ) : readinessChecks.length === 0 ? (
          <p>No readiness checks reported.</p>
        ) : (
          <div className="alertLedger">
            {readinessChecks.map((check) => (
              <div
                className={`alertRow ${check.status === "pass" ? "clear" : "critical"}`}
                key={check.id}
              >
                <strong>
                  {check.status === "pass" ? "✓" : "✗"} {check.label}
                </strong>
                <span>{check.detail}</span>
                <em>{check.status === "pass" ? "Pass" : "Blocking"}</em>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="panel wide">
        <h2>Independent proof verification</h2>
        {!verify ? (
          <p>Verification unavailable (is the API running?).</p>
        ) : (
          <>
            <p>
              <strong>Status:</strong> {verify.passed ? "✅ PASSED" : "❌ FAILED"}
              {verify.reason ? ` — ${verify.reason}` : ""}
            </p>
            {verifyChecks.length > 0 ? (
              <div className="alertLedger">
                {verifyChecks.map((check) => (
                  <div
                    className={`alertRow ${check.status === "pass" ? "clear" : "critical"}`}
                    key={check.name}
                  >
                    <strong>
                      {check.status === "pass" ? "✓" : "✗"} {check.name}
                    </strong>
                    <span>{check.detail}</span>
                    <em>{check.status === "pass" ? "Pass" : "Fail"}</em>
                  </div>
                ))}
              </div>
            ) : null}
          </>
        )}
      </section>

      {scorecard ? (
        <section className={`panel wide statusPanel ${statusClass(scorecard.status)}`}>
          <div>
            <h2>Judge scorecard</h2>
            {scorecard.error ? (
              <p>Failed to load scorecard: {scorecard.error}</p>
            ) : (
              <p>{scorecard.name}</p>
            )}
          </div>
          <div className="metricGrid">
            <div>
              <span>Status</span>
              <strong>{label(scorecard.status)}</strong>
            </div>
            <div>
              <span>Score</span>
              <strong>{scorecard.summary.score_pct}%</strong>
            </div>
            <div>
              <span>Threshold</span>
              <strong>{scorecard.summary.threshold_ready_pct}%</strong>
            </div>
            <div>
              <span>Sections</span>
              <strong>{scorecard.summary.sections}</strong>
            </div>
          </div>
          {scoreSections.length > 0 ? (
            <div className="alertLedger">
              {scoreSections.map((section) => (
                <div className={`alertRow ${statusClass(section.status)}`} key={section.id}>
                  <strong>
                    {section.status === "ready" ? "✓" : "•"} {section.label}
                  </strong>
                  <span>
                    {section.passed_facts}/{section.total_facts} facts · {section.score_pct}% ·
                    weight {section.weight}
                  </span>
                  <em>{label(section.status)}</em>
                </div>
              ))}
            </div>
          ) : null}
        </section>
      ) : null}

      {prizes ? (
        <section className="panel wide statusPanel clear">
          <div>
            <h2>Prize map</h2>
            {prizes.error ? (
              <p>Failed to load prize map: {prizes.error}</p>
            ) : (
              <p>Configured prize claims linked to current evidence surfaces.</p>
            )}
          </div>
          <div className="metricGrid">
            <div>
              <span>Categories</span>
              <strong>{prizes.summary.categories}</strong>
            </div>
            <div>
              <span>Ready</span>
              <strong>{prizes.summary.ready}</strong>
            </div>
            <div>
              <span>Partial</span>
              <strong>{prizes.summary.partial}</strong>
            </div>
          </div>
          {prizeRows.length > 0 ? (
            <div className="alertLedger">
              {prizeRows.map((prize) => (
                <div className={`alertRow ${statusClass(prize.status)}`} key={prize.id}>
                  <strong>
                    {prize.status === "ready" ? "✓" : "•"} {prize.label}
                  </strong>
                  <span>{prize.claim}</span>
                  <em>
                    {label(prize.status)} · {prize.passed_facts}/{prize.total_facts}
                  </em>
                </div>
              ))}
            </div>
          ) : null}
        </section>
      ) : null}

      <section
        className={`panel wide statusPanel ${
          compete
            ? compete.registered && !compete.kill_switch
              ? "clear"
              : "critical"
            : "warning"
        }`}
      >
        <div>
          <p className="eyebrow">Track-1 competition</p>
          <h2>{!compete ? "Compete unavailable" : compete.registered ? "Registered" : "Not registered"}</h2>
        </div>
        {!compete ? (
          <p>Unable to load /compete. Start the API and retry.</p>
        ) : (
          <>
            <div className="stack">
              <CompeteChip ok={compete.registered} text="Registered" />
              <CompeteChip
                ok={compete.eligible_assets > 0}
                text={`${compete.eligible_assets} eligible assets`}
              />
              <CompeteChip ok={compete.daily_trade_satisfied} text="Daily trade" />
              <CompeteChip
                ok={compete.confirmed_trades > 0}
                text={`${compete.confirmed_trades} confirmed trades`}
              />
              <CompeteChip ok={!compete.kill_switch} text="Kill switch armed" />
            </div>
            <div className="metricGrid">
              <div>
                <span>Contract</span>
                <strong>
                  <a className="mono link" href={compete.competition_contract_bsctrace}>
                    {compete.competition_contract}
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
          </>
        )}
      </section>

      <section className="panel wide">
        <h2>Evidence routes</h2>
        <div className="stack">
          {["/readiness", "/proof/verify", "/scorecard", "/prizes", "/compete"].map((path) => (
            <a className="link mono" href={`${API_URL}${path}`} key={path}>
              {path}
            </a>
          ))}
        </div>
      </section>
    </main>
  );
}
