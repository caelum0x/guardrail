// Response types for the Guardrail Alpha API. Decimal-valued fields are
// serialized as strings by the Rust backend to avoid float drift.

export interface HealthResponse {
  ok: boolean;
  database_url?: string;
  events_visible?: number;
  error?: string;
}

export interface StoredEvent {
  id: string;
  run_id: string;
  timestamp: string;
  event_type: string;
  payload_json: Record<string, unknown>;
}

export interface RunPosition {
  symbol: string;
  weight_pct: string;
  value_usd: string;
}

export interface BacktestMetrics {
  total_return_pct: string;
  max_drawdown_pct: string;
  trade_count: number;
  win_rate_pct: string;
  profit_factor: string;
  volatility_pct?: string;
  calmar_ratio?: string;
}

export interface BacktestResponse {
  steps: number;
  preset?: string;
  fear_greed: number;
  starting_nav_usd: string;
  final_nav_usd: string;
  benchmark_return_pct?: string;
  excess_return_pct?: string;
  metrics: BacktestMetrics;
  equity_curve: string[];
  error?: string;
}

export interface WalkForwardWindow {
  window: number;
  fear_greed: number;
  total_return_pct: string;
  benchmark_return_pct: string;
  excess_return_pct: string;
  max_drawdown_pct: string;
  trades: number;
}

export interface WalkForwardResponse {
  windows: WalkForwardWindow[];
  aggregate?: {
    mean_excess_pct: string;
    worst_drawdown_pct: string;
    positive_windows: number;
    total: number;
  };
  preset?: string;
  error?: string;
}

export interface SweepRow {
  fear_greed: number;
  total_return_pct: string;
  benchmark_return_pct: string;
  excess_return_pct: string;
  max_drawdown_pct: string;
  trade_count: number;
}

export interface SweepResponse {
  steps: number;
  preset?: string;
  rows: SweepRow[];
  error?: string;
}

export interface HistoryPoint {
  timestamp: string;
  nav_usd: string;
}

export interface HistoryResponse {
  points: HistoryPoint[];
  count: number;
  error?: string;
}

export interface CompiledPolicyResponse {
  hash?: string;
  policy?: Record<string, unknown>;
  error?: string;
}

export interface GuardrailAlert {
  severity: string;
  kind: string;
  message: string;
}

export interface AlertsResponse {
  status?: string;
  counts: { total?: number; critical?: number; warning?: number };
  alerts: GuardrailAlert[];
  inputs?: Record<string, unknown>;
}

export type StrategyPreset = "conservative" | "balanced" | "aggressive";

// --- /regime --------------------------------------------------------------
export interface RegimeInputs {
  fear_greed: number;
  breadth_pct: string;
  btc_dominance_pct: string;
  median_24h_return: string;
}

export interface RegimeResponse {
  regime: string;
  /** Decimal multiplier, serialized as a string. */
  exposure_multiplier: string;
  inputs: RegimeInputs;
  error?: string;
}

// --- /funding -------------------------------------------------------------
export interface FundingAsset {
  symbol: string;
  price_usd: string;
  funding_rate_proxy: string;
}

export interface FundingResponse {
  assets: FundingAsset[];
  error?: string;
}

// --- /scenarios -----------------------------------------------------------
export interface ScenarioLargestLoss {
  symbol: string | null;
  category?: string;
  pnl_usd: string;
  shock_pct?: string;
}

export interface ScenarioPosition {
  symbol: string;
  category: string;
  weight_pct: string;
  value_usd: string;
  shock_pct: string;
  pnl_usd: string;
  stressed_value_usd: string;
}

export interface ScenarioResult {
  id: string;
  label: string;
  description: string;
  status: string;
  portfolio_pnl_usd: string;
  portfolio_return_pct: string;
  largest_loss: ScenarioLargestLoss;
  positions: ScenarioPosition[];
}

export interface ScenariosResponse {
  report_path?: string;
  universe_path?: string;
  scenarios_path?: string;
  nav_usd: string;
  worst_scenario_id?: string | null;
  worst_pnl_usd: string;
  scenarios: ScenarioResult[];
  error?: string;
}

// --- /readiness -----------------------------------------------------------
export interface ReadinessCheck {
  id: string;
  label: string;
  /** "ready" | "blocking" — emitted by the API per check. */
  status: string;
  detail: string;
}

export interface ReadinessArtifacts {
  report?: string;
  submission_markdown?: string;
  proof?: string;
  alerts?: string;
}

export interface ReadinessResponse {
  status: string;
  blocking: number;
  checks: ReadinessCheck[];
  artifacts: ReadinessArtifacts;
  error?: string;
}

// --- /compete -------------------------------------------------------------
export interface CompeteResponse {
  competition_contract: string;
  competition_contract_bsctrace: string;
  eligible_assets: unknown[];
  registered: boolean;
  competition_tx?: string | null;
  daily_trade_satisfied: boolean;
  confirmed_trades: number;
  kill_switch: boolean;
  error?: string;
}

// --- /skill ---------------------------------------------------------------
export interface SkillResponse {
  name: string;
  skill_yaml: string;
  readme: string;
  examples: unknown;
  error?: string;
}

// --- /signing-policy ------------------------------------------------------
export interface SigningPolicyResponse {
  config_path?: string;
  name: string;
  status: string;
  mode: string;
  chain_id: number;
  headers: { payment: string; accepts: string };
  budget: {
    payment_token: string;
    max_per_call_base_units: string;
    session_budget_base_units: string;
    validity_window_seconds: number;
    max_future_validity_seconds: number;
  };
  summary: {
    allowed_types: number;
    denied_types: number;
    resources: number;
    sample_signed: boolean;
  };
  primary_type_allowlist: string[];
  primary_type_denylist: string[];
  resources: unknown[];
  sample_payment: Record<string, unknown>;
  error?: string;
}

// --- /proof ---------------------------------------------------------------
/**
 * Commitment-bearing report object. Numeric core fields are emitted as-is by
 * the agent; decimal fields are strings. Used for proof re-derivation.
 */
export interface ProofReport {
  run_id?: string;
  cycles?: number;
  final_nav_usd?: string;
  total_drawdown_pct?: string;
  events?: number;
  agent_id?: string;
  wallet_address?: string;
  wallet?: string;
  name?: string;
  policy_hash?: string;
  report_hash?: string;
  address_url?: string;
  registration_tx?: string;
  registration_tx_url?: string;
  [key: string]: unknown;
}

/** The `/proof` envelope. Also matches a bare `data/run_report.json`. */
export interface ProofResponse {
  agent?: string;
  registration_tx?: string | null;
  latest_report?: ProofReport | null;
  run_report?: ProofReport | null;
  source_event_id?: string | null;
  /** Bare run reports carry commitments at the top level. */
  policy_hash?: string;
  wallet_address?: string;
  error?: string;
  [key: string]: unknown;
}
