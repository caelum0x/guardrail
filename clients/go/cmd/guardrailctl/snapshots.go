package main

import (
	"flag"
	"fmt"
	"os"
	"text/tabwriter"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// cmdSnapshots fetches /snapshots and prints the latest run summary plus a
// per-asset latest-price sample. It is offline-safe and always returns exitOK.
func cmdSnapshots(argv []string) int {
	fs := flag.NewFlagSet("snapshots", flag.ContinueOnError)
	common := registerCommon(fs)
	run := fs.String("run", "", "explicit run id to summarize (default: most recent)")
	limit := fs.Int("limit", 0, "cap on per-asset price samples (default: server default)")
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl snapshots [--run ID] [--limit N] [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	ctx, cancel := callContext()
	defer cancel()

	resp, err := common.newClient().Snapshots(ctx, guardrail.SnapshotsParams{Run: *run, Limit: *limit})
	if err != nil {
		fmt.Println(unavailable("snapshots", err))
		return exitOK
	}

	if common.json {
		printJSON(resp)
		return exitOK
	}

	renderSnapshots(resp)
	return exitOK
}

// renderSnapshots prints the discovered runs and the selected run's summary.
func renderSnapshots(resp *guardrail.SnapshotsResponse) {
	fmt.Printf("snapshot directory: %s\n", orDash(resp.Directory))
	fmt.Printf("runs discovered: %d\n", len(resp.Runs))

	if len(resp.Runs) > 0 {
		fmt.Println()
		tw := tabwriter.NewWriter(os.Stdout, 0, 4, 2, ' ', 0)
		fmt.Fprintln(tw, "RUN\tMODIFIED")
		for _, r := range resp.Runs {
			fmt.Fprintf(tw, "%s\t%s\n", r.RunID, formatMillis(r.ModifiedMs))
		}
		_ = tw.Flush()
	}

	if resp.Latest == nil {
		fmt.Println("\n(no run summary available — empty or unavailable snapshot directory)")
		return
	}

	s := resp.Latest
	fmt.Println()
	fmt.Printf("latest run: %s\n", s.RunID)
	fmt.Printf("  cycles=%d skipped=%d\n", s.CycleCount, s.SkippedLines)
	fmt.Printf("  first=%s  last=%s\n", formatMillis(s.FirstTimestampMs), formatMillis(s.LastTimestampMs))

	if len(s.LatestPrices) == 0 {
		fmt.Println("  latest prices: (none)")
		return
	}
	fmt.Println("  latest prices:")
	tw := tabwriter.NewWriter(os.Stdout, 0, 4, 2, ' ', 0)
	fmt.Fprintln(tw, "  SYMBOL\tPRICE_USD")
	for _, p := range s.LatestPrices {
		fmt.Fprintf(tw, "  %s\t%s\n", p.Symbol, p.PriceUSD)
	}
	_ = tw.Flush()
}

// formatMillis renders a nullable epoch-millis timestamp as a UTC RFC3339
// string, or "-" when nil.
func formatMillis(ms *int64) string {
	if ms == nil {
		return "-"
	}
	return time.UnixMilli(*ms).UTC().Format(time.RFC3339)
}
