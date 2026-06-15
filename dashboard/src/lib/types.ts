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
