// Command example is a runnable quickstart for the Guardrail Go SDK.
//
// It targets a local API at http://localhost:8080, calls a few endpoints with a
// short context deadline, and prints results. It exits 0 even when the API is
// unreachable (for example a refused connection) so it is safe to run in CI or
// against an offline backend.
package main

import (
	"context"
	"errors"
	"log"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

func main() {
	log.SetFlags(0)

	client := guardrail.NewClient("http://localhost:8080",
		guardrail.WithTimeout(5*time.Second),
	)

	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()

	if health, err := client.Health(ctx); err != nil {
		logErr("health", err)
	} else {
		log.Printf("health: ok=%t events_visible=%d", health.OK, health.EventsVisible)
	}

	if regime, err := client.Regime(ctx); err != nil {
		logErr("regime", err)
	} else {
		log.Printf("regime: %s (exposure %s, fear/greed %d)",
			regime.Regime, regime.ExposureMultiplier, regime.Inputs.FearGreed)
	}

	if compete, err := client.Compete(ctx); err != nil {
		logErr("compete", err)
	} else {
		log.Printf("compete: registered=%t confirmed_trades=%d daily_trade=%t",
			compete.Registered, compete.ConfirmedTrades, compete.DailyTradeSatisfied)
	}

	bt, err := client.Backtest(ctx, guardrail.BacktestParams{
		Steps:     60,
		FearGreed: 70,
		Preset:    guardrail.PresetBalanced,
	})
	if err != nil {
		logErr("backtest", err)
	} else {
		log.Printf("backtest: total_return=%s trades=%d",
			bt.Metrics.TotalReturnPct, bt.Metrics.TradeCount)
	}

	if hist, err := client.History(ctx); err != nil {
		logErr("history", err)
	} else {
		log.Printf("history: %d points", hist.Count)
	}

	log.Println("done")
}

// logErr reports an endpoint failure without aborting the program. API errors
// surface their status; transport errors (such as a refused connection) are
// logged as warnings.
func logErr(endpoint string, err error) {
	var apiErr *guardrail.APIError
	if errors.As(err, &apiErr) {
		log.Printf("%s: API error %d: %s", endpoint, apiErr.Status, apiErr.Body)
		return
	}
	log.Printf("%s: unavailable: %v", endpoint, err)
}
