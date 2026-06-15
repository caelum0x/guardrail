// Independent, offline proof verifier for the Guardrail BNB AI-Agent.
//
// This mirrors the Python (clients/proof-verifier/verify.py) and Go
// (clients/go/proof.go) verifiers: it takes a `/proof` envelope (or a bare
// `data/run_report.json`) and *independently re-derives* the cryptographic
// commitments the agent claims, comparing them to the claimed values. It also
// validates the competition contract address + explorer URL formats.
//
// Nothing here trusts the agent. Every commitment is recomputed from first
// principles. The SHA-256 implementation uses Node's `crypto` module, loaded
// through a guarded dynamic import so this module still type-checks and bundles
// in environments without Node's `crypto` (e.g. a browser bundler). When no
// SHA-256 backend is available, hash-dependent checks degrade to SKIP rather
// than throwing.
//
// How the agent computes its commitments (mirrored exactly here):
//   * agent_id     = sha256( name + "\x00" + wallet )                  (lowercase hex)
//   * policy_hash  = sha256( raw bytes of the policy file )            (lowercase hex)
//   * report_hash  = sha256( compact JSON of the report "core" object) (lowercase hex)
//       core = {run_id, cycles, final_nav_usd, total_drawdown_pct, events}

import type { ProofReport, ProofResponse } from "./types.js";

// --- Constants mirrored from the Rust workspace (read-only references) -----

/** crates/bnb-agent/src/proof.rs :: BSCSCAN_BASE_URL */
export const BSCSCAN_BASE_URL = "https://bscscan.com";

/** apps/guardrail-api/src/compete.rs :: COMPETITION_CONTRACT */
export const COMPETITION_CONTRACT = "0x212c61b9b72c95d95bf29cf032f5e5635629aed5";

/** apps/guardrail-api/src/compete.rs :: COMPETITION_CONTRACT_BSCTRACE */
export const COMPETITION_CONTRACT_BSCTRACE =
  "https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5";

/** crates/agent-runtime/src/runtime.rs :: report_hash core field order. */
export const REPORT_CORE_FIELDS = [
  "run_id",
  "cycles",
  "final_nav_usd",
  "total_drawdown_pct",
  "events",
] as const;

// Format-validation patterns. ADDRESS_RE accepts the canonical 40-hex address
// plus this repo's 41/42-char vanity placeholder; CANONICAL_ADDRESS_RE enforces
// a strict 20-byte (40-hex) address.
const ADDRESS_RE = /^0x[0-9a-fA-F]{40,42}$/;
const CANONICAL_ADDRESS_RE = /^0x[0-9a-fA-F]{40}$/;
const TX_HASH_RE = /^0x[0-9a-fA-F]{64}$/;
const SHA256_RE = /^[0-9a-f]{64}$/;

// --- Result model ---------------------------------------------------------

export type CheckStatus = "PASS" | "FAIL" | "SKIP";

/** One immutable verification result. */
export interface VerifyCheck {
  name: string;
  status: CheckStatus;
  detail: string;
}

/**
 * Aggregate outcome of verifying a proof. `passed` is true only when no check
 * failed (skips do not, by default, fail the result).
 */
export interface VerifyResult {
  passed: boolean;
  checks: VerifyCheck[];
}

const pass = (name: string, detail: string): VerifyCheck => ({ name, status: "PASS", detail });
const fail = (name: string, detail: string): VerifyCheck => ({ name, status: "FAIL", detail });
const skip = (name: string, detail: string): VerifyCheck => ({ name, status: "SKIP", detail });

// --- SHA-256 backend (guarded dynamic import) -----------------------------

/** Minimal structural view of Node's `crypto` module, so we avoid depending on
 * `@types/node` in this dependency-free client. */
interface NodeHashLike {
  update(data: string, encoding: "utf8"): NodeHashLike;
  digest(encoding: "hex"): string;
}
interface NodeCryptoLike {
  createHash?: (algorithm: "sha256") => NodeHashLike;
}

/**
 * Lowercase hex SHA-256 of a UTF-8 string. Returns `null` when no crypto
 * backend is available (so callers can SKIP rather than throw). Tries the
 * Node `crypto` module first, then the Web Crypto `subtle` API.
 */
export async function sha256Hex(text: string): Promise<string | null> {
  // Node.js path. The module specifier is held in a variable so TypeScript does
  // not resolve `node:crypto`'s types (this client is dependency-free and ships
  // no `@types/node`), and so browser bundlers do not hard-require it. The
  // dynamic shape is narrowed at runtime before use.
  try {
    const spec = "node:crypto";
    const nodeCrypto = (await import(/* @vite-ignore */ spec)) as NodeCryptoLike;
    if (typeof nodeCrypto.createHash === "function") {
      return nodeCrypto.createHash("sha256").update(text, "utf8").digest("hex");
    }
  } catch {
    // Fall through to Web Crypto.
  }

  // Web Crypto path (browsers / Deno / modern runtimes).
  try {
    const subtle = globalThis.crypto?.subtle;
    if (subtle) {
      const data = new TextEncoder().encode(text);
      const digest = await subtle.digest("SHA-256", data);
      return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
    }
  } catch {
    // No backend available.
  }

  return null;
}

// --- Claim extraction (mirrors extract_claims / extractClaims) -------------

interface Claims {
  agent?: string;
  agentId?: string;
  walletAddress?: string;
  policyHash?: string;
  reportHash?: string;
  addressUrl?: string;
  registrationTx?: string;
  registrationTxUrl?: string;
  reportCoreSource: Record<string, unknown>;
}

function str(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function extractClaims(proof: ProofResponse): Claims {
  const summary: ProofReport = (proof.latest_report ?? {}) as ProofReport;
  const report: ProofReport = (proof.run_report ?? {}) as ProofReport;

  // If the top-level doc is itself a bare report, treat it as both.
  const bareTop =
    !proof.latest_report && !proof.run_report && proof.policy_hash != null
      ? (proof as unknown as ProofReport)
      : undefined;
  const primary: ProofReport = bareTop ?? summary;
  const fallback: ProofReport = bareTop ?? report;

  const pick = (key: keyof ProofReport): unknown =>
    primary[key] ?? fallback[key] ?? (proof as Record<string, unknown>)[key as string];

  const source =
    Object.keys(summary).length > 0
      ? summary
      : Object.keys(report).length > 0
        ? report
        : (proof as Record<string, unknown>);

  return {
    agent: str(proof.agent) ?? str(pick("name")),
    agentId: str(pick("agent_id")),
    walletAddress: str(pick("wallet_address")) ?? str(pick("wallet")),
    policyHash: str(pick("policy_hash")),
    reportHash: str(pick("report_hash")),
    addressUrl: str(pick("address_url")),
    registrationTx: str(proof.registration_tx ?? undefined) ?? str(pick("registration_tx")),
    registrationTxUrl: str(pick("registration_tx_url")),
    reportCoreSource: source as Record<string, unknown>,
  };
}

// --- Re-derivation helpers ------------------------------------------------

/**
 * Re-derive report_hash by reproducing serde_json's compact JSON of the core
 * object in field order. Returns `null` when a core field is missing or no
 * crypto backend exists.
 */
async function reportHashFor(source: Record<string, unknown>): Promise<string | null> {
  for (const field of REPORT_CORE_FIELDS) {
    if (!(field in source)) {
      return null;
    }
  }
  // serde_json compact form: no spaces after separators, insertion order.
  const parts = REPORT_CORE_FIELDS.map(
    (field) => `${JSON.stringify(field)}:${JSON.stringify(source[field])}`,
  );
  const canonical = `{${parts.join(",")}}`;
  return sha256Hex(canonical);
}

/** Re-derive agent_id = sha256(name + 0x00 + wallet). */
async function agentIdFor(name: string, wallet: string): Promise<string | null> {
  return sha256Hex(`${name}${String.fromCharCode(0)}${wallet}`);
}

// --- Verification stages --------------------------------------------------

function verifyWallet(c: Claims): VerifyCheck {
  const wallet = c.walletAddress;
  if (!wallet) {
    return fail("wallet_address", "proof carries no wallet_address");
  }
  if (CANONICAL_ADDRESS_RE.test(wallet)) {
    return pass("wallet_address", `valid 20-byte EVM address: ${wallet}`);
  }
  if (ADDRESS_RE.test(wallet)) {
    return pass("wallet_address", `0x-prefixed hex address (demo/vanity placeholder length): ${wallet}`);
  }
  return fail("wallet_address", `wallet_address is not a 0x-prefixed hex EVM address: ${JSON.stringify(wallet)}`);
}

async function verifyPolicyHash(c: Claims, policyRaw?: string): Promise<VerifyCheck> {
  const claimed = c.policyHash;
  if (!claimed) {
    return skip("policy_hash", "no policy_hash claimed in proof; skipped");
  }
  if (!SHA256_RE.test(claimed)) {
    return fail("policy_hash", `claimed policy_hash is not a 64-char lowercase hex digest: ${JSON.stringify(claimed)}`);
  }
  if (policyRaw == null) {
    return skip("policy_hash", "no policy file content supplied to recompute against; skipped");
  }
  const recomputed = await sha256Hex(policyRaw);
  if (recomputed == null) {
    return skip("policy_hash", "no SHA-256 backend available to recompute policy_hash; skipped");
  }
  if (recomputed === claimed) {
    return pass("policy_hash", `recomputed sha256 of supplied policy content matches claimed ${claimed}`);
  }
  return fail("policy_hash", `recomputed ${recomputed} != claimed ${claimed}`);
}

async function verifyReportHash(c: Claims): Promise<VerifyCheck> {
  const claimed = c.reportHash;
  if (!claimed) {
    return skip("report_hash", "no report_hash claimed (bare run reports omit it); skipped");
  }
  if (!SHA256_RE.test(claimed)) {
    return fail("report_hash", `claimed report_hash is not a 64-char lowercase hex digest: ${JSON.stringify(claimed)}`);
  }
  const missing = REPORT_CORE_FIELDS.filter((field) => !(field in c.reportCoreSource));
  if (missing.length > 0) {
    return fail("report_hash", `cannot re-derive report_hash: proof is missing core field(s) ${JSON.stringify(missing)}`);
  }
  const recomputed = await reportHashFor(c.reportCoreSource);
  if (recomputed == null) {
    return skip("report_hash", "no SHA-256 backend available to recompute report_hash; skipped");
  }
  if (recomputed === claimed) {
    return pass("report_hash", `recomputed sha256 over {${REPORT_CORE_FIELDS.join(", ")}} matches claimed ${claimed}`);
  }
  return fail("report_hash", `recomputed ${recomputed} != claimed ${claimed}`);
}

async function verifyAgentId(c: Claims): Promise<VerifyCheck> {
  const claimed = c.agentId;
  if (!claimed) {
    return skip("agent_id", "no agent_id claimed in proof; skipped");
  }
  if (!c.agent || !c.walletAddress) {
    return skip("agent_id", "cannot re-derive agent_id without both agent name and wallet; skipped");
  }
  const recomputed = await agentIdFor(c.agent, c.walletAddress);
  if (recomputed == null) {
    return skip("agent_id", "no SHA-256 backend available to recompute agent_id; skipped");
  }
  if (recomputed === claimed) {
    return pass("agent_id", `recomputed sha256(name\\x00wallet) matches claimed ${claimed}`);
  }
  return fail("agent_id", `recomputed ${recomputed} != claimed ${claimed} (name=${JSON.stringify(c.agent)}, wallet=${JSON.stringify(c.walletAddress)})`);
}

function verifyAddressUrl(c: Claims): VerifyCheck {
  const url = c.addressUrl;
  if (!url) {
    return skip("address_url", "no address_url claimed (bare run reports omit it); skipped");
  }
  if (!c.walletAddress) {
    return fail("address_url", "address_url present but wallet_address missing");
  }
  const expected = `${BSCSCAN_BASE_URL}/address/${c.walletAddress}`;
  if (url === expected) {
    return pass("address_url", `BscScan address URL well-formed: ${url}`);
  }
  return fail("address_url", `address_url ${JSON.stringify(url)} != expected ${JSON.stringify(expected)}`);
}

function verifyRegistrationTx(c: Claims): VerifyCheck {
  const tx = c.registrationTx;
  if (!tx) {
    return skip("registration_tx", "no registration_tx anchored yet (optional, set out-of-band); skipped");
  }
  if (!TX_HASH_RE.test(tx)) {
    return fail("registration_tx", `registration_tx is not a 0x + 64-hex tx hash: ${JSON.stringify(tx)}`);
  }
  if (c.registrationTxUrl) {
    const expected = `${BSCSCAN_BASE_URL}/tx/${tx}`;
    if (c.registrationTxUrl !== expected) {
      return fail("registration_tx", `registration_tx_url ${JSON.stringify(c.registrationTxUrl)} != expected ${JSON.stringify(expected)}`);
    }
  }
  return pass("registration_tx", `valid tx hash format: ${tx}`);
}

/** Validate the fixed competition contract address + explorer URL formats. */
function verifyCompetitionContract(): VerifyCheck[] {
  const addrOk = ADDRESS_RE.test(COMPETITION_CONTRACT);
  const addrCheck = addrOk
    ? pass("competition_contract_format", `competition contract is a valid EVM address: ${COMPETITION_CONTRACT}`)
    : fail("competition_contract_format", `competition contract is malformed: ${COMPETITION_CONTRACT}`);

  const expectedExplorer = `https://bsctrace.com/address/${COMPETITION_CONTRACT}`;
  const explorerOk = COMPETITION_CONTRACT_BSCTRACE === expectedExplorer;
  const explorerCheck = explorerOk
    ? pass("competition_contract_explorer_url", `explorer URL embeds the contract: ${COMPETITION_CONTRACT_BSCTRACE}`)
    : fail("competition_contract_explorer_url", `explorer URL ${JSON.stringify(COMPETITION_CONTRACT_BSCTRACE)} does not embed ${JSON.stringify(COMPETITION_CONTRACT)}`);

  return [addrCheck, explorerCheck];
}

// --- Public API -----------------------------------------------------------

export interface VerifyOptions {
  /**
   * Raw policy-file content (exact bytes as a UTF-8 string) to recompute
   * policy_hash against. When omitted, the policy_hash check is skipped.
   */
  policyRaw?: string;
}

/**
 * Verify a `/proof` envelope (or bare run report) entirely offline. Re-derives
 * every applicable commitment and validates the competition contract metadata.
 * Pure aside from the guarded SHA-256 import; never throws on missing data.
 */
export async function verifyProof(
  proof: ProofResponse,
  options: VerifyOptions = {},
): Promise<VerifyResult> {
  const claims = extractClaims(proof);
  const checks: VerifyCheck[] = [
    verifyWallet(claims),
    await verifyPolicyHash(claims, options.policyRaw),
    await verifyReportHash(claims),
    await verifyAgentId(claims),
    verifyAddressUrl(claims),
    verifyRegistrationTx(claims),
    ...verifyCompetitionContract(),
  ];
  const passed = checks.every((c) => c.status !== "FAIL");
  return { passed, checks };
}

/** Render a human-readable PASS/FAIL report mirroring the Python verifier. */
export function renderReport(result: VerifyResult, source = "proof"): string {
  const rule = "============================================================";
  const lines = [
    rule,
    " Guardrail BNB AI-Agent — Independent Proof Verification",
    rule,
    ` proof source : ${source}`,
    "",
  ];
  for (const check of result.checks) {
    lines.push(` [${check.status}] ${check.name}`);
    lines.push(`        ${check.detail}`);
  }
  const passed = result.checks.filter((c) => c.status === "PASS").length;
  const failed = result.checks.filter((c) => c.status === "FAIL").length;
  const skipped = result.checks.filter((c) => c.status === "SKIP").length;
  lines.push("");
  lines.push("------------------------------------------------------------");
  lines.push(` RESULT: ${failed === 0 ? "PASS" : "FAIL"}  (${passed} passed, ${failed} failed, ${skipped} skipped)`);
  lines.push(rule);
  return lines.join("\n");
}
