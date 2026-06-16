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
  onchain_configured?: boolean;
  checks?: VerifyCheck[];
}

function statusIcon(status: string): string {
  if (status === "pass") return "✅";
  if (status === "skipped") return "⏭️";
  return "❌";
}

function CheckList({ checks }: { checks: VerifyCheck[] }) {
  return (
    <ul>
      {checks.map((c) => (
        <li key={c.name}>
          {statusIcon(c.status)} <strong>{c.name}</strong>: {c.detail}
        </li>
      ))}
    </ul>
  );
}

export default async function ProofPage() {
  const proof = await getJsonOrNull<ProofResponse>("/proof");
  const verify = await getJsonOrNull<VerifyResponse>("/proof/verify");
  const checks = verify?.checks ?? [];
  const onchainChecks = checks.filter((c) => c.name.startsWith("onchain"));
  const offlineChecks = checks.filter((c) => !c.name.startsWith("onchain"));

  const erc8004 = proof?.erc8004 ?? null;

  return (
    <main className="grid">
      <ProofCard proof={proof} />

      {erc8004 ? (
        <section className="card">
          <h2>Agent Identity (ERC-8004)</h2>
          <p>
            On-chain agent identity via the Trust Wallet Agent Kit. Keys stay in
            TWAK; the identity NFT is minted on BSC under self-custody — verifiable
            on-chain, not self-attested.
          </p>
          <p>
            <strong>Status:</strong>{" "}
            {erc8004.anchored ? "✅ Anchored on-chain" : "⏳ Not yet anchored"}
          </p>
          <ul>
            <li>
              <strong>Identity registry:</strong>{" "}
              <a href={erc8004.registry_url} target="_blank" rel="noreferrer">
                {erc8004.registry}
              </a>
            </li>
            {erc8004.agent_id ? (
              <li>
                <strong>agentId:</strong> <code>{erc8004.agent_id}</code>
              </li>
            ) : null}
            {erc8004.identity_tx && erc8004.identity_tx_url ? (
              <li>
                <strong>Mint tx:</strong>{" "}
                <a href={erc8004.identity_tx_url} target="_blank" rel="noreferrer">
                  {erc8004.identity_tx}
                </a>
              </li>
            ) : null}
          </ul>
          {!erc8004.anchored ? (
            <p>
              Anchoring runs in a live, autonomous session with{" "}
              <code>GUARDRAIL_ANCHOR_IDENTITY=1</code> — gated on a wallet
              password, so paper/demo runs never mint.
            </p>
          ) : null}
        </section>
      ) : null}

      <section className="card">
        <h2>Independent Verification</h2>
        <p>
          Recomputes the agent&apos;s policy hash and validates the competition
          contract, wallet, and BscScan/registration formats — the same checks the
          standalone <code>clients/proof-verifier</code> performs, served over the
          read-only API. These run fully offline.
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
            {offlineChecks.length > 0 ? <CheckList checks={offlineChecks} /> : null}
          </>
        )}
      </section>

      {verify ? (
        <section className="card">
          <h2>On-chain Verification</h2>
          <p>
            Read-only BSC JSON-RPC checks (<code>eth_chainId</code>,{" "}
            <code>eth_getCode</code>, <code>eth_getTransactionReceipt</code>) that
            confirm the competition contract is deployed and the registration
            transaction was actually mined — verifiable, not self-attested. No keys,
            no signing.
          </p>
          {verify.onchain_configured ? (
            onchainChecks.length > 0 ? (
              <CheckList checks={onchainChecks} />
            ) : (
              <p>No on-chain checks returned.</p>
            )
          ) : (
            <p>
              ⏭️ On-chain checks skipped — set <code>BSC_RPC_URL</code> on the API to
              verify against the live chain. The offline proof above still passes.
            </p>
          )}
        </section>
      ) : null}
    </main>
  );
}
