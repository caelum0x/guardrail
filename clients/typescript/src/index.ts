// Typed client for the Guardrail Alpha read-only API.
//
// Dependency-free: uses the global `fetch` (Node 18+ or the browser). Every
// method maps to one API route. The API is read-only — this client never
// mutates agent state.

import type {
  AlertsResponse,
  BacktestResponse,
  CompeteResponse,
  CompiledPolicyResponse,
  FundingResponse,
  HealthResponse,
  HistoryResponse,
  ProofResponse,
  ReadinessResponse,
  RegimeResponse,
  ScenariosResponse,
  SigningPolicyResponse,
  SkillResponse,
  StoredEvent,
  StrategyPreset,
  SweepResponse,
  WalkForwardResponse,
} from "./types.js";

export * from "./types.js";
export * from "./proof.js";

export interface GuardrailClientOptions {
  /** Base URL of the API, e.g. http://localhost:8080 */
  baseUrl?: string;
  /** Optional fetch override (for tests or custom agents). */
  fetchImpl?: typeof fetch;
}

export interface BacktestParams {
  steps?: number;
  fearGreed?: number;
  preset?: StrategyPreset;
}

export interface WalkForwardParams {
  windows?: number;
  steps?: number;
  preset?: StrategyPreset;
}

export interface SweepParams {
  steps?: number;
  fearGreed?: number[];
  preset?: StrategyPreset;
}

export interface IndicatorParams {
  /** Asset symbol to chart (default "WBNB"). */
  symbol?: string;
  /** Number of synthetic close steps (clamped 10..500). */
  steps?: number;
}

export interface OptimizeParams {
  /** Comma-separated symbols (default CAKE,WBNB,BTCB). */
  symbols?: string[];
  /** Scores aligned to `symbols`. */
  scores?: number[];
  /** Volatilities aligned to `symbols`. */
  vols?: number[];
}

const DEFAULT_BASE_URL = "http://localhost:8080";

export class GuardrailClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;

  constructor(options: GuardrailClientOptions = {}) {
    this.baseUrl = (options.baseUrl ?? DEFAULT_BASE_URL).replace(/\/+$/, "");
    const f = options.fetchImpl ?? globalThis.fetch;
    if (!f) {
      throw new Error("No fetch implementation available; pass fetchImpl");
    }
    this.fetchImpl = f.bind(globalThis);
  }

  private async getJson<T>(path: string): Promise<T> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`, { headers: { Accept: "application/json" } });
    if (!res.ok) {
      throw new Error(`GET ${path} failed: ${res.status} ${res.statusText}`);
    }
    return (await res.json()) as T;
  }

  private async getText(path: string): Promise<string> {
    const res = await this.fetchImpl(`${this.baseUrl}${path}`);
    if (!res.ok) {
      throw new Error(`GET ${path} failed: ${res.status} ${res.statusText}`);
    }
    return res.text();
  }

  // --- Status & state -------------------------------------------------------
  health(): Promise<HealthResponse> {
    return this.getJson<HealthResponse>("/health");
  }
  cockpit(): Promise<Record<string, unknown>> {
    return this.getJson("/cockpit");
  }
  portfolio(): Promise<Record<string, unknown>> {
    return this.getJson("/portfolio");
  }
  risk(): Promise<Record<string, unknown>> {
    return this.getJson("/risk");
  }
  alerts(): Promise<AlertsResponse> {
    return this.getJson<AlertsResponse>("/alerts");
  }
  proof(): Promise<ProofResponse> {
    return this.getJson<ProofResponse>("/proof");
  }
  events(): Promise<{ events: StoredEvent[] }> {
    return this.getJson("/events");
  }
  history(): Promise<HistoryResponse> {
    return this.getJson<HistoryResponse>("/history");
  }
  /** Prometheus exposition text from the API /metrics route. */
  metrics(): Promise<string> {
    return this.getText("/metrics");
  }

  // --- Research -------------------------------------------------------------
  backtest(params: BacktestParams = {}): Promise<BacktestResponse> {
    const q = new URLSearchParams();
    if (params.steps != null) q.set("steps", String(params.steps));
    if (params.fearGreed != null) q.set("fear_greed", String(params.fearGreed));
    if (params.preset) q.set("preset", params.preset);
    return this.getJson<BacktestResponse>(`/backtest?${q.toString()}`);
  }

  walkforward(params: WalkForwardParams = {}): Promise<WalkForwardResponse> {
    const q = new URLSearchParams();
    if (params.windows != null) q.set("windows", String(params.windows));
    if (params.steps != null) q.set("steps", String(params.steps));
    if (params.preset) q.set("preset", params.preset);
    return this.getJson<WalkForwardResponse>(`/walkforward?${q.toString()}`);
  }

  sweep(params: SweepParams = {}): Promise<SweepResponse> {
    const q = new URLSearchParams();
    if (params.steps != null) q.set("steps", String(params.steps));
    if (params.fearGreed?.length) q.set("fear_greed", params.fearGreed.join(","));
    if (params.preset) q.set("preset", params.preset);
    return this.getJson<SweepResponse>(`/sweep?${q.toString()}`);
  }

  trades(): Promise<Record<string, unknown>> {
    return this.getJson("/trades");
  }
  signals(): Promise<Record<string, unknown>> {
    return this.getJson("/signals");
  }
  readiness(): Promise<ReadinessResponse> {
    return this.getJson<ReadinessResponse>("/readiness");
  }
  exposure(): Promise<Record<string, unknown>> {
    return this.getJson("/exposure");
  }
  briefing(): Promise<Record<string, unknown>> {
    return this.getJson("/briefing");
  }
  budget(): Promise<Record<string, unknown>> {
    return this.getJson("/budget");
  }
  heartbeat(): Promise<Record<string, unknown>> {
    return this.getJson("/heartbeat");
  }
  costs(): Promise<Record<string, unknown>> {
    return this.getJson("/costs");
  }
  drift(): Promise<Record<string, unknown>> {
    return this.getJson("/drift");
  }
  exitTriggers(): Promise<Record<string, unknown>> {
    return this.getJson("/exit-triggers");
  }
  liquidity(): Promise<Record<string, unknown>> {
    return this.getJson("/liquidity");
  }
  quotes(): Promise<Record<string, unknown>> {
    return this.getJson("/quotes");
  }
  watchlist(): Promise<Record<string, unknown>> {
    return this.getJson("/watchlist");
  }
  rebalance(): Promise<Record<string, unknown>> {
    return this.getJson("/rebalance");
  }
  scenarios(): Promise<ScenariosResponse> {
    return this.getJson<ScenariosResponse>("/scenarios");
  }

  // --- Market & research ----------------------------------------------------
  assets(): Promise<Record<string, unknown>> {
    return this.getJson("/assets");
  }
  trending(): Promise<Record<string, unknown>> {
    return this.getJson("/trending");
  }
  regime(): Promise<RegimeResponse> {
    return this.getJson<RegimeResponse>("/regime");
  }
  funding(): Promise<FundingResponse> {
    return this.getJson<FundingResponse>("/funding");
  }
  mandates(): Promise<Record<string, unknown>> {
    return this.getJson("/mandates");
  }
  experiments(): Promise<Record<string, unknown>> {
    return this.getJson("/experiments");
  }

  /** Deterministic synthetic indicators for a symbol (``/indicators``). */
  indicators(params: IndicatorParams = {}): Promise<Record<string, unknown>> {
    const q = new URLSearchParams();
    if (params.symbol) q.set("symbol", params.symbol);
    if (params.steps != null) q.set("steps", String(params.steps));
    const qs = q.toString();
    return this.getJson(qs ? `/indicators?${qs}` : "/indicators");
  }

  /** Portfolio weight optimization for a basket (``/optimize``). */
  optimize(params: OptimizeParams = {}): Promise<Record<string, unknown>> {
    const q = new URLSearchParams();
    if (params.symbols?.length) q.set("symbols", params.symbols.join(","));
    if (params.scores?.length) q.set("scores", params.scores.join(","));
    if (params.vols?.length) q.set("vols", params.vols.join(","));
    const qs = q.toString();
    return this.getJson(qs ? `/optimize?${qs}` : "/optimize");
  }

  // --- Governance & catalog -------------------------------------------------
  universe(): Promise<Record<string, unknown>> {
    return this.getJson("/universe");
  }
  config(): Promise<Record<string, unknown>> {
    return this.getJson("/config");
  }
  ops(): Promise<Record<string, unknown>> {
    return this.getJson("/ops");
  }
  policy(): Promise<Record<string, unknown>> {
    return this.getJson("/policy");
  }
  signingPolicy(): Promise<SigningPolicyResponse> {
    return this.getJson<SigningPolicyResponse>("/signing-policy");
  }
  walletControls(): Promise<Record<string, unknown>> {
    return this.getJson("/wallet-controls");
  }
  playbook(): Promise<Record<string, unknown>> {
    return this.getJson("/playbook");
  }
  prizes(): Promise<Record<string, unknown>> {
    return this.getJson("/prizes");
  }
  commerce(): Promise<Record<string, unknown>> {
    return this.getJson("/commerce");
  }
  sdkCatalog(): Promise<Record<string, unknown>> {
    return this.getJson("/sdk-catalog");
  }
  bnbSdk(): Promise<Record<string, unknown>> {
    return this.getJson("/bnb-sdk");
  }

  // --- Reporting & proof ----------------------------------------------------
  report(): Promise<Record<string, unknown>> {
    return this.getJson("/report");
  }
  /** Human-readable Markdown report (``/report/markdown``). */
  reportMarkdown(): Promise<string> {
    return this.getText("/report/markdown");
  }
  /** Competition submission Markdown (``/export/submission.md``). */
  exportSubmissionMarkdown(): Promise<string> {
    return this.getText("/export/submission.md");
  }
  scorecard(): Promise<Record<string, unknown>> {
    return this.getJson("/scorecard");
  }
  auditManifest(): Promise<Record<string, unknown>> {
    return this.getJson("/audit-manifest");
  }
  skill(): Promise<SkillResponse> {
    return this.getJson<SkillResponse>("/skill");
  }
  compete(): Promise<CompeteResponse> {
    return this.getJson<CompeteResponse>("/compete");
  }
  jobSimulator(): Promise<Record<string, unknown>> {
    return this.getJson("/job-simulator");
  }

  // --- Agent identity -------------------------------------------------------
  agentServices(): Promise<Record<string, unknown>> {
    return this.getJson("/agent-services");
  }
  agentCard(): Promise<Record<string, unknown>> {
    return this.getJson("/agent-card");
  }
  /** ERC-8004 style well-known agent card (``/.well-known/agent-card.json``). */
  wellKnownAgentCard(): Promise<Record<string, unknown>> {
    return this.getJson("/.well-known/agent-card.json");
  }

  // --- Policy ---------------------------------------------------------------
  /** Compile a natural-language mandate into a validated policy + hash. */
  compilePolicy(mandate: string): Promise<CompiledPolicyResponse> {
    return this.getJson<CompiledPolicyResponse>(
      `/policy/compile?mandate=${encodeURIComponent(mandate)}`,
    );
  }
}

export default GuardrailClient;
