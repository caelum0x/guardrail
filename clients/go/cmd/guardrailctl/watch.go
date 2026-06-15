package main

import (
	"context"
	"flag"
	"fmt"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// minInterval guards against a zero or absurdly small polling interval that
// would busy-loop the API.
const minInterval = 1 * time.Second

// cmdWatch polls /compete and /regime on an interval and prints a refreshing
// one-line status. With --once it prints a single tick and returns. It stops
// cleanly on SIGINT/SIGTERM. It always returns exitOK so it is offline-safe.
func cmdWatch(argv []string) int {
	fs := flag.NewFlagSet("watch", flag.ContinueOnError)
	common := registerCommon(fs)
	intervalSec := fs.Int("interval", 5, "polling interval in seconds")
	once := fs.Bool("once", false, "print a single status tick and exit")
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl watch [--interval N] [--once] [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	interval := time.Duration(*intervalSec) * time.Second
	if interval < minInterval {
		interval = minInterval
	}

	// Cancel the whole watch loop on the first interrupt signal so an in-flight
	// poll is abandoned promptly and the tool exits cleanly.
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	client := common.newClient()

	// First tick immediately so the operator sees output without waiting.
	printTick(ctx, client, common.json)
	if *once {
		fmt.Println()
		return exitOK
	}

	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			// Move past the in-place status line before exiting.
			fmt.Println()
			return exitOK
		case <-ticker.C:
			printTick(ctx, client, common.json)
		}
	}
}

// printTick fetches /compete and /regime once and renders a single status line.
// JSON mode prints a discrete line per tick; table mode rewrites the current
// terminal line in place with a carriage return so the status appears to
// refresh. It never fails: unreachable endpoints render as a notice.
func printTick(ctx context.Context, client *guardrail.Client, asJSON bool) {
	callCtx, cancel := context.WithTimeout(ctx, requestTimeout)
	defer cancel()

	stamp := time.Now().Format("15:04:05")
	regime, regimeErr := client.Regime(callCtx)
	compete, competeErr := client.Compete(callCtx)

	if asJSON {
		printTickJSON(stamp, regime, regimeErr, compete, competeErr)
		return
	}

	line := renderStatusLine(stamp, regime, regimeErr, compete, competeErr)
	// \r returns to column 0; trailing spaces clear any longer previous line.
	fmt.Printf("\r%-110s", line)
}

// renderStatusLine builds the one-line table-mode status string.
func renderStatusLine(stamp string, regime *guardrail.RegimeResponse, regimeErr error, compete *guardrail.CompeteResponse, competeErr error) string {
	var b strings.Builder
	fmt.Fprintf(&b, "[%s] ", stamp)

	if regimeErr != nil {
		b.WriteString("regime=offline")
	} else {
		fmt.Fprintf(&b, "regime=%s exposure=%s f/g=%d",
			orDash(regime.Regime), orDash(regime.ExposureMultiplier), regime.Inputs.FearGreed)
	}

	b.WriteString("  |  ")

	if competeErr != nil {
		b.WriteString("compete=offline")
	} else {
		fmt.Fprintf(&b, "registered=%t trades=%d daily=%t kill=%t",
			compete.Registered, compete.ConfirmedTrades, compete.DailyTradeSatisfied, compete.KillSwitch)
	}
	return b.String()
}

// printTickJSON emits one JSON object for the tick, folding API failures into
// an "offline" marker so the stream stays valid and parseable.
func printTickJSON(stamp string, regime *guardrail.RegimeResponse, regimeErr error, compete *guardrail.CompeteResponse, competeErr error) {
	tick := map[string]any{"time": stamp}
	if regimeErr != nil {
		tick["regime"] = map[string]any{"status": "offline", "error": regimeErr.Error()}
	} else {
		tick["regime"] = regime
	}
	if competeErr != nil {
		tick["compete"] = map[string]any{"status": "offline", "error": competeErr.Error()}
	} else {
		tick["compete"] = compete
	}
	printJSON(tick)
}

// orDash renders an empty string as a dash so columns never collapse.
func orDash(s string) string {
	if strings.TrimSpace(s) == "" {
		return "-"
	}
	return s
}
