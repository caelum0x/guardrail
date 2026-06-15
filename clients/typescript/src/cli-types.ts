// Response types for the operator-CLI surface of the Guardrail Alpha API.
//
// These mirror the Go SDK (clients/go/types.go) field-for-field so the Node CLI
// can render the same /ensemble, /journal, /snapshots, /skills, /skills/{id},
// and /proof/verify views. Decimal-valued fields are serialized as strings by
// the Rust backend to avoid float drift.

// --- /ensemble ------------------------------------------------------------

/** One Track-2 strategy Skill identified in the ensemble. */
export interface EnsembleSkill {
  id: string;
  label: string;
}

/** One per-regime row of the static weight table; weights map skill id -> weight. */
export interface EnsembleRegime {
  regime: string;
  weights: Record<string, number>;
}

/**
 * Regime-routed skill-ensemble view from `/ensemble`. `current_regime` and
 * `active_weights` are null when the live snapshot could not be built.
 */
export interface EnsembleResponse {
  name: string;
  version: string;
  reserve_symbol: string;
  max_risk_allocation_pct: number;
  current_regime?: string | null;
  active_weights?: Record<string, number> | null;
  skills: EnsembleSkill[];
  regimes: EnsembleRegime[];
  error?: string;
}

// --- /journal -------------------------------------------------------------

/** One scored asset within a journal cycle. */
export interface JournalAsset {
  symbol: string;
  score: number;
}

/** One proposed rebalance order; `amount_usd` is nullable in the payload. */
export interface JournalOrder {
  from: string;
  to: string;
  amount_usd?: number | null;
}

/** Risk-engine outcomes for a journal cycle. */
export interface JournalRisk {
  approved: number;
  clipped: number;
  rejected: number;
  rejection_reasons: string[];
}

/** One decision cycle of the verifiable-autonomy narrative. */
export interface JournalCycle {
  index: number;
  run_id: string;
  regime: string;
  started_at: string;
  ended_at: string;
  headline: string;
  top_assets: JournalAsset[];
  orders: JournalOrder[];
  risk: JournalRisk;
  confirmed_trades: number;
  ending_nav?: string | null;
  positions?: number | null;
}

/** Per-cycle decision journal from `/journal`. */
export interface JournalResponse {
  total_events: number;
  total_cycles: number;
  run_ids: string[];
  confirmed_trades_total: number;
  cycles: JournalCycle[];
  error?: string;
}

// --- /snapshots -----------------------------------------------------------

/** One discovered run file; `modified_ms` is nullable. */
export interface SnapshotRunFile {
  run_id: string;
  modified_ms?: number | null;
}

/** One per-asset latest-price sample (price serialized as a string). */
export interface SnapshotPriceSample {
  symbol: string;
  price_usd: string;
}

/** Compact summary of one run's snapshot history; timestamps are nullable. */
export interface SnapshotRunSummary {
  run_id: string;
  cycle_count: number;
  skipped_lines: number;
  first_timestamp_ms?: number | null;
  last_timestamp_ms?: number | null;
  latest_prices: SnapshotPriceSample[];
}

/** Market-snapshot history view from `/snapshots`; `latest` is null when empty. */
export interface SnapshotsResponse {
  directory: string;
  runs: SnapshotRunFile[];
  latest?: SnapshotRunSummary | null;
}

// --- /skills + /skills/{id} ----------------------------------------------

/** One published Track-2 Skill in the `/skills` catalog. */
export interface SkillCatalogEntry {
  id: string;
  name: string;
  path?: string;
  summary?: string;
  regimes: string[];
  inputs?: string[];
  eligible_universe_size?: number;
  examples_count?: number;
  spec_file?: string;
}

/** Track-2 Skill catalog from `/skills`. */
export interface SkillsResponse {
  index_path: string;
  count: number;
  ids: string[];
  skills: SkillCatalogEntry[];
}

/**
 * Per-skill detail from `/skills/{id}`. `error` is set (with the offending id)
 * when the requested skill is not found or the catalog could not be loaded.
 */
export interface SkillDetail {
  id?: string;
  name?: string;
  summary?: string;
  description?: string;
  regimes?: string[];
  inputs?: string[];
  eligible_universe_size?: number;
  examples_count?: number;
  examples_on_disk?: number;
  spec_file?: string;
  spec_sections?: string[];
  error?: string;
}

// --- /proof/verify --------------------------------------------------------

/**
 * One server-side verification check from `/proof/verify`. Unlike the offline
 * verifier's PASS/FAIL/SKIP, the server reports a lowercase "pass"/"fail".
 */
export interface ServerCheck {
  name: string;
  status: string;
  detail: string;
}

/** A candidate policy file + the sha256 the server recomputed for it. */
export interface RecomputedPolicyHash {
  file: string;
  sha256: string;
}

/**
 * Server-side proof verification result from `/proof/verify`. `reason` is set
 * (with an empty `checks` list) when no run report exists yet.
 */
export interface ProofVerifyResponse {
  passed: boolean;
  reason?: string;
  report_path?: string;
  recomputed_policy_hashes?: RecomputedPolicyHash[];
  checks: ServerCheck[];
}
