import { ProofCard } from "../../components/ProofCard";
import { getJsonOrNull } from "../../lib/api";
import type { ProofResponse } from "../../lib/types";

interface VerifyCheck {
  name: string;
  status: string;
  detail: string;
}

interface VerifyResponse {
  passed?: boolean;
  reason?: string;
  checks?: VerifyCheck[];
}

export default async function ProofPage() {
  const proof = await getJsonOrNull<ProofResponse>("/proof");
  const verify = await getJsonOrNull<VerifyResponse>("/proof/verify");
  const checks = verify?.checks ?? [];

  return (
    <main className="grid">
      <ProofCard proof={proof} />

      <section className="card">
        <h2>Independent Verification</h2>
        <p>
          Recomputes the agent&apos;s policy hash and validates the competition
          contract, wallet, and BscScan/registration formats — the same checks the
          standalone <code>clients/proof-verifier</code> performs, served over the
          read-only API.
        </p>
        {!verify ? (
          <p>Verification unavailable (is the API running?).</p>
        ) : (
          <>
            <p>
              <strong>Status:</strong>{" "}
              {verify.passed ? "✅ PASSED" : "❌ FAILED"}
              {verify.reason ? ` — ${verify.reason}` : ""}
            </p>
            {checks.length > 0 ? (
              <ul>
                {checks.map((c) => (
                  <li key={c.name}>
                    {c.status === "pass" ? "✅" : "❌"} <strong>{c.name}</strong>:{" "}
                    {c.detail}
                  </li>
                ))}
              </ul>
            ) : null}
          </>
        )}
      </section>
    </main>
  );
}
