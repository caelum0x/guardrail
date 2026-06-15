export type HealthResponse = {
  ok: boolean;
  database_url?: string;
  events_visible?: number;
  error?: string;
};

export type StoredEvent = {
  id: string;
  run_id: string;
  timestamp: string;
  event_type: string;
  payload_json: Record<string, unknown>;
};

export type LatestReport = {
  run_id?: string;
  cycles?: number;
  events?: number;
  final_nav_usd?: string;
  total_drawdown_pct?: string;
  agent_id?: string;
  wallet_address?: string;
  policy_hash?: string;
  report_hash?: string;
  address_url?: string;
  registration_tx_url?: string;
};

export type RunPosition = {
  symbol?: string;
  weight_pct?: string;
  value_usd?: string;
};

export type RunReport = {
  run_id?: string;
  mode?: string;
  updated_ms?: number;
  wallet_address?: string;
  nav_usd?: string;
  starting_nav_usd?: string;
  total_drawdown_pct?: string;
  regime?: string;
  kill_switch?: boolean;
  positions?: RunPosition[];
  trades?: unknown[];
  events?: number;
  policy_hash?: string;
};

export type PortfolioResponse = {
  latest?: {
    nav_usd?: string;
    positions?: number;
  } | null;
  source_event_id?: string | null;
};

export type TradesResponse = {
  trades: StoredEvent[];
};

export type EventsResponse = {
  events: StoredEvent[];
  error?: string;
};

export type SignalsResponse = {
  regime?: Record<string, unknown> | null;
  target?: Record<string, unknown> | null;
};

export type RiskResponse = {
  kill_switch: boolean;
  events: StoredEvent[];
};

export type AlertSeverity = "info" | "warning" | "critical";

export type GuardrailAlert = {
  kind: string;
  severity: AlertSeverity;
  message: string;
};

export type AlertsResponse = {
  status: "clear" | "warning" | "critical";
  counts: {
    critical: number;
    warning: number;
    total: number;
  };
  alerts: GuardrailAlert[];
  inputs: {
    report_age_seconds: number;
    total_drawdown_pct: number;
    drawdown_soft_limit_pct: number;
    drawdown_hard_limit_pct: number;
    latest_slippage_pct: number;
    slippage_limit_pct: number;
    kill_switch: boolean;
    daily_trade_executed: boolean;
    events_visible: number;
    trades_visible: number;
    report_path: string;
  };
};

export type ReadinessCheck = {
  id: string;
  label: string;
  status: "pass" | "blocking";
  detail: string;
};

export type ReadinessResponse = {
  status: "ready" | "blocking";
  blocking: number;
  checks: ReadinessCheck[];
  artifacts: Record<string, string>;
};

export type ProofResponse = {
  agent: string;
  registration_tx?: string | null;
  latest_report?: LatestReport | null;
  run_report?: RunReport | null;
  source_event_id?: string | null;
};

export type CockpitResponse = {
  health: HealthResponse;
  latest_report?: LatestReport | null;
  run_report?: RunReport | null;
  portfolio?: {
    nav_usd?: string;
    positions?: number;
  } | null;
  regime?: Record<string, unknown> | null;
  target?: Record<string, unknown> | null;
  risk: {
    kill_switch: boolean;
    recent_decisions: number;
  };
  execution: {
    confirmed_txs: number;
    latest_tx?: Record<string, unknown> | null;
  };
  activity: StoredEvent[];
};

export type FileValue<T> = {
  path: string;
  ok: boolean;
  value?: T;
  error?: string;
};

export type PolicyResponse = {
  production: FileValue<Record<string, unknown>>;
  paper: FileValue<Record<string, unknown>>;
  schema: FileValue<Record<string, unknown>>;
  enforcement: Record<string, unknown>;
};

export type UniverseAsset = {
  symbol?: string;
  cmc_id?: number;
  chain_id?: number;
  contract_address?: string;
  decimals?: number;
  category?: string;
  enabled?: boolean;
  min_liquidity_usd?: number;
  min_volume_24h_usd?: number;
};

export type UniverseResponse = {
  path: string;
  enabled_assets: number;
  assets: UniverseAsset[] | { error: string };
};

export type ConfigResponse = {
  runtime: {
    paper: string;
    production: string;
    backtest: string;
  };
  strategy_weights: FileValue<Record<string, unknown>>;
  execution_limits: FileValue<Record<string, unknown>>;
  asset_categories: FileValue<Record<string, unknown>>;
  secrets_template: string;
  environment: {
    database_url: string;
    report_path: string;
  };
};

export type OpsResponse = {
  mode: string;
  operator_commands: Array<{ name: string; command: string }>;
  http_surfaces: string[];
  docker: Record<string, string>;
  safety: string[];
};

export type EnsembleSkill = {
  id: string;
  label: string;
};

/** Per-skill weight object keyed by skill id (e.g. `{ "trend-breakout-momentum": 0.5 }`). */
export type EnsembleWeights = Record<string, number>;

export type EnsembleRegimeRow = {
  regime: string;
  weights: EnsembleWeights;
};

export type EnsembleResponse = {
  name?: string;
  version?: string;
  reserve_symbol?: string;
  max_risk_allocation_pct?: number;
  current_regime?: string | null;
  active_weights?: EnsembleWeights | null;
  skills?: EnsembleSkill[];
  regimes?: EnsembleRegimeRow[];
  error?: string;
};

export type JournalScoredAsset = {
  symbol: string;
  score: number;
};

export type JournalOrder = {
  from: string;
  to: string;
  amount_usd?: number | null;
};

export type JournalRisk = {
  approved: number;
  clipped: number;
  rejected: number;
  rejection_reasons: string[];
};

export type JournalCycle = {
  index: number;
  run_id?: string;
  regime: string;
  started_at?: string;
  ended_at?: string;
  headline?: string;
  top_assets?: JournalScoredAsset[];
  orders?: JournalOrder[];
  risk?: JournalRisk;
  confirmed_trades?: number;
  ending_nav?: string | null;
  positions?: number | null;
};

export type JournalResponse = {
  total_events?: number;
  total_cycles?: number;
  run_ids?: string[];
  confirmed_trades_total?: number;
  cycles?: JournalCycle[];
  error?: string;
};

/** One discovered market-snapshot run file (`GET /snapshots`). */
export type SnapshotRunFile = {
  run_id: string;
  /** Last-modified time in ms since the Unix epoch, when available. */
  modified_ms?: number | null;
};

/** Per-asset latest-price sample drawn from the last snapshot line. */
export type SnapshotPriceSample = {
  symbol: string;
  price_usd: string;
};

/** Compact summary of a single run's snapshot history. */
export type SnapshotRunSummary = {
  run_id: string;
  cycle_count: number;
  skipped_lines: number;
  first_timestamp_ms?: number | null;
  last_timestamp_ms?: number | null;
  latest_prices: SnapshotPriceSample[];
};

/** Top-level response for `GET /snapshots`. */
export type SnapshotsResponse = {
  /** Resolved snapshot directory that was inspected. */
  directory: string;
  /** All discovered run files, newest first. */
  runs: SnapshotRunFile[];
  /** Summary of the selected run (most recent by default), if one exists. */
  latest?: SnapshotRunSummary | null;
};

/** Independent proof-verification result (`GET /proof/verify`). */
export type ProofVerifyCheck = {
  name: string;
  status: string;
  detail: string;
};

export type ProofVerifyResponse = {
  passed?: boolean;
  reason?: string;
  checks?: ProofVerifyCheck[];
};

/** One judge-scorecard section (`GET /scorecard`). */
export type ScorecardSection = {
  id: string;
  label: string;
  weight: number;
  status: string;
  passed_facts: number;
  total_facts: number;
  score_pct: number;
  evidence_routes: string[];
  required_facts: string[];
};

export type ScorecardResponse = {
  name: string;
  status: string;
  summary: {
    score_pct: number;
    threshold_ready_pct: number;
    earned_weight: number;
    total_weight: number;
    sections: number;
  };
  sections: ScorecardSection[];
  error?: string;
};

/** One prize-lane claim mapping (`GET /prizes`). */
export type Prize = {
  id: string;
  label: string;
  claim: string;
  evidence_paths: string[];
  passed_facts: number;
  total_facts: number;
  status: "ready" | "partial" | string;
};

export type PrizesResponse = {
  summary: {
    categories: number;
    ready: number;
    partial: number;
  };
  prizes: Prize[];
  error?: string;
};

/** Track-1 competition readiness (`GET /compete`). */
export type CompeteResponse = {
  competition_contract: string;
  competition_contract_bsctrace: string;
  eligible_assets: number;
  registered: boolean;
  competition_tx: string | null;
  daily_trade_satisfied: boolean;
  confirmed_trades: number;
  kill_switch: boolean;
};
