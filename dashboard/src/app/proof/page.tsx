import { ProofCard } from "../../components/ProofCard";
import { getJsonOrNull } from "../../lib/api";
import type { ProofResponse } from "../../lib/types";

export default async function ProofPage() {
  const proof = await getJsonOrNull<ProofResponse>("/proof");
  return (
    <main className="grid">
      <ProofCard proof={proof} />
    </main>
  );
}
