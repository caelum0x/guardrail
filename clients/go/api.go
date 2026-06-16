package guardrail

import (
	"context"
	"net/url"
	"strconv"
	"strings"
)

// --- Parameter option types -------------------------------------------------

// BacktestParams configures a /backtest request. Zero-valued fields are
// omitted, so the server applies its own defaults.
type BacktestParams struct {
	Steps     int            // number of synthetic steps (omitted when 0)
	FearGreed int            // fear/greed index (omitted when 0)
	Preset    StrategyPreset // risk preset (omitted when empty)
}

// WalkForwardParams configures a /walkforward request.
type WalkForwardParams struct {
	Windows int            // number of rolling windows (omitted when 0)
	Steps   int            // steps per window (omitted when 0)
	Preset  StrategyPreset // risk preset (omitted when empty)
}

// SweepParams configures a /sweep request.
type SweepParams struct {
	Steps     int            // steps per scenario (omitted when 0)
	FearGreed []int          // fear/greed values to sweep (omitted when empty)
	Preset    StrategyPreset // risk preset (omitted when empty)
}

// IndicatorParams configures an /indicators request.
type IndicatorParams struct {
	Symbol string // asset symbol (omitted when empty)
	Steps  int    // synthetic close steps (omitted when 0)
}

// OptimizeParams configures an /optimize request.
type OptimizeParams struct {
	Symbols []string  // basket symbols (omitted when empty)
	Scores  []float64 // scores aligned to Symbols (omitted when empty)
	Vols    []float64 // volatilities aligned to Symbols (omitted when empty)
}

// --- Status & state ---------------------------------------------------------

// Health returns API + database status (/health).
func (c *Client) Health(ctx context.Context) (*HealthResponse, error) {
	out := &HealthResponse{}
	if err := c.do(ctx, "", "/health", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Cockpit returns the aggregated live view (/cockpit).
func (c *Client) Cockpit(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/cockpit")
}

// Portfolio returns the latest reconciliation (/portfolio).
func (c *Client) Portfolio(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/portfolio")
}

// Risk returns risk events + kill switch state (/risk).
func (c *Client) Risk(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/risk")
}

// Alerts returns the evaluated alert set (/alerts).
func (c *Client) Alerts(ctx context.Context) (*AlertsResponse, error) {
	out := &AlertsResponse{}
	if err := c.do(ctx, "", "/alerts", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Events returns the recent event log (/events).
func (c *Client) Events(ctx context.Context) (*EventsResponse, error) {
	out := &EventsResponse{}
	if err := c.do(ctx, "", "/events", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// History returns the NAV equity series (/history).
func (c *Client) History(ctx context.Context) (*HistoryResponse, error) {
	out := &HistoryResponse{}
	if err := c.do(ctx, "", "/history", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Metrics returns Prometheus exposition text (/metrics).
func (c *Client) Metrics(ctx context.Context) (string, error) {
	return c.doText(ctx, "/metrics", nil)
}

// Readiness returns the readiness probe (/readiness).
func (c *Client) Readiness(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/readiness")
}

// Trades returns recent trades (/trades).
func (c *Client) Trades(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/trades")
}

// Signals returns the latest signals (/signals).
func (c *Client) Signals(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/signals")
}

// Exposure returns portfolio exposure (/exposure).
func (c *Client) Exposure(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/exposure")
}

// Briefing returns the operator briefing (/briefing).
func (c *Client) Briefing(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/briefing")
}

// Budget returns budget status (/budget).
func (c *Client) Budget(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/budget")
}

// Heartbeat returns heartbeat status (/heartbeat).
func (c *Client) Heartbeat(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/heartbeat")
}

// Costs returns cost accounting (/costs).
func (c *Client) Costs(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/costs")
}

// Drift returns allocation drift (/drift).
func (c *Client) Drift(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/drift")
}

// ExitTriggers returns exit triggers (/exit-triggers).
func (c *Client) ExitTriggers(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/exit-triggers")
}

// Liquidity returns the liquidity view (/liquidity).
func (c *Client) Liquidity(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/liquidity")
}

// Quotes returns the latest quotes (/quotes).
func (c *Client) Quotes(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/quotes")
}

// Watchlist returns the watchlist (/watchlist).
func (c *Client) Watchlist(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/watchlist")
}

// Rebalance returns the rebalance plan (/rebalance).
func (c *Client) Rebalance(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/rebalance")
}

// Scenarios returns stress scenarios (/scenarios).
func (c *Client) Scenarios(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/scenarios")
}

// --- Research ---------------------------------------------------------------

// Backtest runs a strategy-vs-benchmark backtest (/backtest).
func (c *Client) Backtest(ctx context.Context, params BacktestParams) (*BacktestResponse, error) {
	q := url.Values{}
	if params.Steps != 0 {
		q.Set("steps", strconv.Itoa(params.Steps))
	}
	if params.FearGreed != 0 {
		q.Set("fear_greed", strconv.Itoa(params.FearGreed))
	}
	if params.Preset != "" {
		q.Set("preset", string(params.Preset))
	}
	out := &BacktestResponse{}
	if err := c.do(ctx, "", "/backtest", q, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// WalkForward runs rolling walk-forward windows (/walkforward).
func (c *Client) WalkForward(ctx context.Context, params WalkForwardParams) (*WalkForwardResponse, error) {
	q := url.Values{}
	if params.Windows != 0 {
		q.Set("windows", strconv.Itoa(params.Windows))
	}
	if params.Steps != 0 {
		q.Set("steps", strconv.Itoa(params.Steps))
	}
	if params.Preset != "" {
		q.Set("preset", string(params.Preset))
	}
	out := &WalkForwardResponse{}
	if err := c.do(ctx, "", "/walkforward", q, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Sweep runs a sentiment comparison sweep (/sweep).
func (c *Client) Sweep(ctx context.Context, params SweepParams) (*SweepResponse, error) {
	q := url.Values{}
	if params.Steps != 0 {
		q.Set("steps", strconv.Itoa(params.Steps))
	}
	if len(params.FearGreed) > 0 {
		q.Set("fear_greed", joinInts(params.FearGreed))
	}
	if params.Preset != "" {
		q.Set("preset", string(params.Preset))
	}
	out := &SweepResponse{}
	if err := c.do(ctx, "", "/sweep", q, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Optimize computes portfolio weights for a basket (/optimize).
func (c *Client) Optimize(ctx context.Context, params OptimizeParams) (map[string]any, error) {
	q := url.Values{}
	if len(params.Symbols) > 0 {
		q.Set("symbols", strings.Join(params.Symbols, ","))
	}
	if len(params.Scores) > 0 {
		q.Set("scores", joinFloats(params.Scores))
	}
	if len(params.Vols) > 0 {
		q.Set("vols", joinFloats(params.Vols))
	}
	return c.getMapQuery(ctx, "/optimize", q)
}

// Indicators returns deterministic synthetic indicators for a symbol
// (/indicators).
func (c *Client) Indicators(ctx context.Context, params IndicatorParams) (map[string]any, error) {
	q := url.Values{}
	if params.Symbol != "" {
		q.Set("symbol", params.Symbol)
	}
	if params.Steps != 0 {
		q.Set("steps", strconv.Itoa(params.Steps))
	}
	return c.getMapQuery(ctx, "/indicators", q)
}

// --- Market & research ------------------------------------------------------

// Assets returns the tracked assets (/assets).
func (c *Client) Assets(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/assets")
}

// Trending returns trending assets (/trending).
func (c *Client) Trending(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/trending")
}

// Regime returns the market regime classification (/regime).
func (c *Client) Regime(ctx context.Context) (*RegimeResponse, error) {
	out := &RegimeResponse{}
	if err := c.do(ctx, "", "/regime", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Funding returns funding rates (/funding).
func (c *Client) Funding(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/funding")
}

// Ensemble returns the regime-routed skill-ensemble view (/ensemble): the
// current classified regime, the weights active for it, and the full static
// per-regime weight table.
func (c *Client) Ensemble(ctx context.Context) (*EnsembleResponse, error) {
	out := &EnsembleResponse{}
	if err := c.do(ctx, "", "/ensemble", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Journal returns the per-cycle decision journal (/journal), reconstructing the
// verifiable-autonomy narrative from the append-only event log.
func (c *Client) Journal(ctx context.Context) (*JournalResponse, error) {
	out := &JournalResponse{}
	if err := c.do(ctx, "", "/journal", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// SnapshotsParams configures a /snapshots request. Zero-valued fields are
// omitted, so the server applies its own defaults.
type SnapshotsParams struct {
	Run   string // explicit run id to summarize (omitted when empty; default: most recent)
	Limit int    // cap on per-asset price samples (omitted when 0)
}

// Snapshots returns the persisted market-snapshot history (/snapshots): the
// discovered run files plus a compact summary of the selected run (most recent
// by default) with a per-asset latest-price sample.
func (c *Client) Snapshots(ctx context.Context, params SnapshotsParams) (*SnapshotsResponse, error) {
	q := url.Values{}
	if params.Run != "" {
		q.Set("run", params.Run)
	}
	if params.Limit != 0 {
		q.Set("limit", strconv.Itoa(params.Limit))
	}
	out := &SnapshotsResponse{}
	if err := c.do(ctx, "", "/snapshots", q, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Skills returns the Track-2 Skill catalog (/skills): the published skills with
// their ids, names, and regimes.
func (c *Client) Skills(ctx context.Context) (*SkillsResponse, error) {
	out := &SkillsResponse{}
	if err := c.do(ctx, "", "/skills", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// SkillByID returns the per-skill detail (/skills/{id}): the catalog entry plus
// the loaded spec summary. An unknown id yields a response with Error set rather
// than an HTTP error. The id is path-escaped before use.
func (c *Client) SkillByID(ctx context.Context, id string) (*SkillDetail, error) {
	out := &SkillDetail{}
	if err := c.do(ctx, "", "/skills/"+url.PathEscape(id), nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// Mandates returns the mandate catalog (/mandates).
func (c *Client) Mandates(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/mandates")
}

// Experiments returns the experiment log (/experiments).
func (c *Client) Experiments(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/experiments")
}

// --- Governance & catalog ---------------------------------------------------

// Universe returns the trading universe (/universe).
func (c *Client) Universe(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/universe")
}

// Config returns the config inventory (/config).
func (c *Client) Config(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/config")
}

// Ops returns ops status (/ops).
func (c *Client) Ops(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/ops")
}

// Policy returns the active policy (/policy).
func (c *Client) Policy(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/policy")
}

// Playbook returns the operator playbook (/playbook).
func (c *Client) Playbook(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/playbook")
}

// Prizes returns the prize catalog (/prizes).
func (c *Client) Prizes(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/prizes")
}

// Commerce returns the commerce view (/commerce).
func (c *Client) Commerce(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/commerce")
}

// SDKCatalog returns the SDK catalog (/sdk-catalog).
func (c *Client) SDKCatalog(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/sdk-catalog")
}

// BNBSdk returns BNB SDK metadata (/bnb-sdk).
func (c *Client) BNBSdk(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/bnb-sdk")
}

// --- Reporting & proof ------------------------------------------------------

// Report returns the structured report JSON (/report).
func (c *Client) Report(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/report")
}

// ReportMarkdown returns the human-readable Markdown report (/report/markdown).
func (c *Client) ReportMarkdown(ctx context.Context) (string, error) {
	return c.doText(ctx, "/report/markdown", nil)
}

// ExportSubmissionMarkdown returns the competition submission Markdown
// (/export/submission.md).
func (c *Client) ExportSubmissionMarkdown(ctx context.Context) (string, error) {
	return c.doText(ctx, "/export/submission.md", nil)
}

// Scorecard returns the judge scorecard (/scorecard).
func (c *Client) Scorecard(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/scorecard")
}

// Skill returns the skill descriptor (/skill).
func (c *Client) Skill(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/skill")
}

// Compete returns the Track-1 competition readiness payload (/compete).
func (c *Client) Compete(ctx context.Context) (*CompeteResponse, error) {
	out := &CompeteResponse{}
	if err := c.do(ctx, "", "/compete", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// JobSimulator returns the job simulator view (/job-simulator).
func (c *Client) JobSimulator(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/job-simulator")
}

// --- Agent identity ---------------------------------------------------------

// AgentCard returns the agent card (/agent-card).
func (c *Client) AgentCard(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/agent-card")
}

// WellKnownAgentCard returns the ERC-8004 well-known agent card
// (/.well-known/agent-card.json).
func (c *Client) WellKnownAgentCard(ctx context.Context) (map[string]any, error) {
	return c.getMap(ctx, "/.well-known/agent-card.json")
}

// --- Policy -----------------------------------------------------------------

// CompilePolicy compiles a natural-language mandate into a validated policy +
// hash (/policy/compile).
func (c *Client) CompilePolicy(ctx context.Context, mandate string) (*CompiledPolicyResponse, error) {
	q := url.Values{}
	q.Set("mandate", mandate)
	out := &CompiledPolicyResponse{}
	if err := c.do(ctx, "", "/policy/compile", q, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}

// --- Internal helpers -------------------------------------------------------

// getMap decodes a JSON object response into a map.
func (c *Client) getMap(ctx context.Context, path string) (map[string]any, error) {
	return c.getMapQuery(ctx, path, nil)
}

// getMapQuery decodes a JSON object response into a map, applying query params.
func (c *Client) getMapQuery(ctx context.Context, path string, q url.Values) (map[string]any, error) {
	out := map[string]any{}
	if err := c.do(ctx, "", path, q, nil, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// TA computes a technical indicator over a close-price series (/ta).
// indicator is one of sma|ema|rsi|macd|bollinger; period/mult are optional (0 = omit).
func (c *Client) TA(ctx context.Context, indicator string, series []float64, period int, mult float64) (map[string]any, error) {
	q := url.Values{}
	q.Set("indicator", indicator)
	q.Set("series", joinFloats(series))
	if period != 0 {
		q.Set("period", strconv.Itoa(period))
	}
	if mult != 0 {
		q.Set("mult", strconv.FormatFloat(mult, 'f', -1, 64))
	}
	return c.getMapQuery(ctx, "/ta", q)
}

// Fees estimates the all-in cost of a swap (/fees). Pass nil for all defaults;
// keys: notional_usd, quantity, side, gas_units, gas_price_gwei, native_usd,
// pool_liquidity_usd, linear_slippage_bps, protocol_fee_bps.
func (c *Client) Fees(ctx context.Context, params map[string]string) (map[string]any, error) {
	q := url.Values{}
	for k, v := range params {
		q.Set(k, v)
	}
	return c.getMapQuery(ctx, "/fees", q)
}

// Sizer computes a position size by method (/sizer): fixed_fractional|vol_target|kelly.
func (c *Client) Sizer(ctx context.Context, method string, params map[string]string) (map[string]any, error) {
	q := url.Values{}
	q.Set("method", method)
	for k, v := range params {
		q.Set(k, v)
	}
	return c.getMapQuery(ctx, "/sizer", q)
}

// CMCCapabilities returns the CMC data->capability lineage (/cmc/capabilities).
func (c *Client) CMCCapabilities(ctx context.Context) (map[string]any, error) {
	return c.getMapQuery(ctx, "/cmc/capabilities", nil)
}

func joinInts(values []int) string {
	parts := make([]string, len(values))
	for i, v := range values {
		parts[i] = strconv.Itoa(v)
	}
	return strings.Join(parts, ",")
}

func joinFloats(values []float64) string {
	parts := make([]string, len(values))
	for i, v := range values {
		parts[i] = strconv.FormatFloat(v, 'f', -1, 64)
	}
	return strings.Join(parts, ",")
}
