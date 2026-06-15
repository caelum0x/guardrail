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
