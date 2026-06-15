package guardrail

import "encoding/json"

// StrategyPreset selects a risk profile for research endpoints.
type StrategyPreset string

// Supported strategy presets.
const (
	PresetConservative StrategyPreset = "conservative"
	PresetBalanced     StrategyPreset = "balanced"
	PresetAggressive   StrategyPreset = "aggressive"
)

// Decimal-valued fields are serialized as strings by the Rust backend to avoid
// float drift, so they are typed as string here.

// HealthResponse is the payload from /health.
type HealthResponse struct {
	OK            bool   `json:"ok"`
	DatabaseURL   string `json:"database_url,omitempty"`
	EventsVisible int    `json:"events_visible,omitempty"`
	Error         string `json:"error,omitempty"`
}

// StoredEvent is a single persisted agent event from /events.
type StoredEvent struct {
	ID          string          `json:"id"`
	RunID       string          `json:"run_id"`
	Timestamp   string          `json:"timestamp"`
	EventType   string          `json:"event_type"`
	PayloadJSON json.RawMessage `json:"payload_json"`
}

// EventsResponse wraps the /events list.
type EventsResponse struct {
	Events []StoredEvent `json:"events"`
}

// HistoryPoint is one NAV sample from /history.
type HistoryPoint struct {
	Timestamp string `json:"timestamp"`
	NavUSD    string `json:"nav_usd"`
}

// HistoryResponse is the NAV equity series from /history.
type HistoryResponse struct {
	Points []HistoryPoint `json:"points"`
	Count  int            `json:"count"`
	Error  string         `json:"error,omitempty"`
}

// RegimeInputs are the classifier inputs reported by /regime.
type RegimeInputs struct {
	FearGreed       int    `json:"fear_greed"`
	BreadthPct      string `json:"breadth_pct"`
	BTCDominancePct string `json:"btc_dominance_pct"`
	Median24hReturn string `json:"median_24h_return"`
}

// RegimeResponse is the market regime classification from /regime.
type RegimeResponse struct {
	Regime             string       `json:"regime"`
	ExposureMultiplier string       `json:"exposure_multiplier"`
	Inputs             RegimeInputs `json:"inputs"`
	Error              string       `json:"error,omitempty"`
}

// CompeteResponse is the Track-1 competition readiness payload from /compete.
type CompeteResponse struct {
	CompetitionContract         string `json:"competition_contract"`
	CompetitionContractBsctrace string `json:"competition_contract_bsctrace"`
	EligibleAssets              int    `json:"eligible_assets"`
	Registered                  bool   `json:"registered"`
	CompetitionTx               string `json:"competition_tx"`
	DailyTradeSatisfied         bool   `json:"daily_trade_satisfied"`
	ConfirmedTrades             int    `json:"confirmed_trades"`
	KillSwitch                  bool   `json:"kill_switch"`
	Error                       string `json:"error,omitempty"`
}

// GuardrailAlert is a single evaluated alert.
type GuardrailAlert struct {
	Severity string `json:"severity"`
	Kind     string `json:"kind"`
	Message  string `json:"message"`
}

// AlertCounts summarizes alert severities.
type AlertCounts struct {
	Total    int `json:"total"`
	Critical int `json:"critical"`
	Warning  int `json:"warning"`
}

// AlertsResponse is the evaluated alert set from /alerts.
type AlertsResponse struct {
	Status string           `json:"status,omitempty"`
	Counts AlertCounts      `json:"counts"`
	Alerts []GuardrailAlert `json:"alerts"`
	Inputs map[string]any   `json:"inputs,omitempty"`
}

// BacktestMetrics are the headline backtest statistics.
type BacktestMetrics struct {
	TotalReturnPct string `json:"total_return_pct"`
	MaxDrawdownPct string `json:"max_drawdown_pct"`
	TradeCount     int    `json:"trade_count"`
	WinRatePct     string `json:"win_rate_pct"`
	ProfitFactor   string `json:"profit_factor"`
	VolatilityPct  string `json:"volatility_pct,omitempty"`
	CalmarRatio    string `json:"calmar_ratio,omitempty"`
}

// BacktestResponse is the strategy-vs-benchmark backtest from /backtest.
type BacktestResponse struct {
	Steps              int             `json:"steps"`
	Preset             string          `json:"preset,omitempty"`
	FearGreed          int             `json:"fear_greed"`
	StartingNavUSD     string          `json:"starting_nav_usd"`
	FinalNavUSD        string          `json:"final_nav_usd"`
	BenchmarkReturnPct string          `json:"benchmark_return_pct,omitempty"`
	ExcessReturnPct    string          `json:"excess_return_pct,omitempty"`
	Metrics            BacktestMetrics `json:"metrics"`
	EquityCurve        []string        `json:"equity_curve"`
	Error              string          `json:"error,omitempty"`
}

// WalkForwardWindow is one rolling window result.
type WalkForwardWindow struct {
	Window             int    `json:"window"`
	FearGreed          int    `json:"fear_greed"`
	TotalReturnPct     string `json:"total_return_pct"`
	BenchmarkReturnPct string `json:"benchmark_return_pct"`
	ExcessReturnPct    string `json:"excess_return_pct"`
	MaxDrawdownPct     string `json:"max_drawdown_pct"`
	Trades             int    `json:"trades"`
}

// WalkForwardAggregate summarizes a walk-forward run.
type WalkForwardAggregate struct {
	MeanExcessPct    string `json:"mean_excess_pct"`
	WorstDrawdownPct string `json:"worst_drawdown_pct"`
	PositiveWindows  int    `json:"positive_windows"`
	Total            int    `json:"total"`
}

// WalkForwardResponse is the rolling walk-forward result from /walkforward.
type WalkForwardResponse struct {
	Windows   []WalkForwardWindow   `json:"windows"`
	Aggregate *WalkForwardAggregate `json:"aggregate,omitempty"`
	Preset    string                `json:"preset,omitempty"`
	Error     string                `json:"error,omitempty"`
}

// SweepRow is one sentiment-comparison row.
type SweepRow struct {
	FearGreed          int    `json:"fear_greed"`
	TotalReturnPct     string `json:"total_return_pct"`
	BenchmarkReturnPct string `json:"benchmark_return_pct"`
	ExcessReturnPct    string `json:"excess_return_pct"`
	MaxDrawdownPct     string `json:"max_drawdown_pct"`
	TradeCount         int    `json:"trade_count"`
}

// SweepResponse is the sentiment comparison sweep from /sweep.
type SweepResponse struct {
	Steps  int        `json:"steps"`
	Preset string     `json:"preset,omitempty"`
	Rows   []SweepRow `json:"rows"`
	Error  string     `json:"error,omitempty"`
}

// CompiledPolicyResponse is the result of compiling a mandate via
// /policy/compile.
type CompiledPolicyResponse struct {
	Hash   string         `json:"hash,omitempty"`
	Policy map[string]any `json:"policy,omitempty"`
	Error  string         `json:"error,omitempty"`
}

// EnsembleSkill identifies one Track-2 strategy Skill in the ensemble.
type EnsembleSkill struct {
	ID    string `json:"id"`
	Label string `json:"label"`
}

// EnsembleRegime is a single per-regime row of the static weight table. Weights
// maps each Skill id to its blend weight for that regime.
type EnsembleRegime struct {
	Regime  string             `json:"regime"`
	Weights map[string]float64 `json:"weights"`
}

// EnsembleResponse is the regime-routed skill ensemble view from /ensemble. It
// pairs the static per-regime weight table with the currently classified
// regime and the weights active for it (both nil when the live snapshot could
// not be built).
type EnsembleResponse struct {
	Name                 string             `json:"name"`
	Version              string             `json:"version"`
	ReserveSymbol        string             `json:"reserve_symbol"`
	MaxRiskAllocationPct float64            `json:"max_risk_allocation_pct"`
	CurrentRegime        *string            `json:"current_regime"`
	ActiveWeights        map[string]float64 `json:"active_weights"`
	Skills               []EnsembleSkill    `json:"skills"`
	Regimes              []EnsembleRegime   `json:"regimes"`
	Error                string             `json:"error,omitempty"`
}

// JournalAsset is one scored asset within a journal cycle.
type JournalAsset struct {
	Symbol string  `json:"symbol"`
	Score  float64 `json:"score"`
}

// JournalOrder is one proposed rebalance order within a journal cycle. AmountUSD
// is nullable in the source payload, so it is a pointer.
type JournalOrder struct {
	From      string   `json:"from"`
	To        string   `json:"to"`
	AmountUSD *float64 `json:"amount_usd"`
}

// JournalRisk summarizes risk-engine outcomes for a journal cycle.
type JournalRisk struct {
	Approved         int      `json:"approved"`
	Clipped          int      `json:"clipped"`
	Rejected         int      `json:"rejected"`
	RejectionReasons []string `json:"rejection_reasons"`
}

// JournalCycle is one decision cycle of the verifiable-autonomy narrative.
// EndingNav and Positions are nullable in the source payload.
type JournalCycle struct {
	Index           int            `json:"index"`
	RunID           string         `json:"run_id"`
	Regime          string         `json:"regime"`
	StartedAt       string         `json:"started_at"`
	EndedAt         string         `json:"ended_at"`
	Headline        string         `json:"headline"`
	TopAssets       []JournalAsset `json:"top_assets"`
	Orders          []JournalOrder `json:"orders"`
	Risk            JournalRisk    `json:"risk"`
	ConfirmedTrades int            `json:"confirmed_trades"`
	EndingNav       *string        `json:"ending_nav"`
	Positions       *int           `json:"positions"`
}

// JournalResponse is the per-cycle decision journal from /journal.
type JournalResponse struct {
	TotalEvents          int            `json:"total_events"`
	TotalCycles          int            `json:"total_cycles"`
	RunIDs               []string       `json:"run_ids"`
	ConfirmedTradesTotal int            `json:"confirmed_trades_total"`
	Cycles               []JournalCycle `json:"cycles"`
	Error                string         `json:"error,omitempty"`
}
