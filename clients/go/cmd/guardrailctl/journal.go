package main

import (
	"flag"
	"fmt"
	"os"
	"strings"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// cmdJournal fetches /journal and prints a compact per-cycle decision journal.
// It is offline-safe and always returns exitOK.
func cmdJournal(argv []string) int {
	fs := flag.NewFlagSet("journal", flag.ContinueOnError)
	common := registerCommon(fs)
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl journal [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	ctx, cancel := callContext()
	defer cancel()

	resp, err := common.newClient().Journal(ctx)
	if err != nil {
		fmt.Println(unavailable("journal", err))
		return exitOK
	}

	if common.json {
		printJSON(resp)
		return exitOK
	}

	renderJournal(resp)
	return exitOK
}

// renderJournal prints the journal summary header followed by one compact block
// per decision cycle.
func renderJournal(resp *guardrail.JournalResponse) {
	fmt.Printf("decision journal: %d cycles, %d events, %d confirmed trades\n",
		resp.TotalCycles, resp.TotalEvents, resp.ConfirmedTradesTotal)
	if len(resp.RunIDs) > 0 {
		fmt.Printf("runs: %s\n", strings.Join(resp.RunIDs, ", "))
	}

	if len(resp.Cycles) == 0 {
		fmt.Println("\n(no cycles recorded — empty or unavailable event log)")
		return
	}

	for _, cycle := range resp.Cycles {
		fmt.Println()
		renderCycle(cycle)
	}
}

// renderCycle prints a single decision cycle as a labelled, indented block.
func renderCycle(cycle guardrail.JournalCycle) {
	fmt.Printf("#%d  regime=%s  %s -> %s\n",
		cycle.Index, orDash(cycle.Regime), orDash(cycle.StartedAt), orDash(cycle.EndedAt))
	if cycle.RunID != "" {
		fmt.Printf("    run: %s\n", cycle.RunID)
	}
	if strings.TrimSpace(cycle.Headline) != "" {
		fmt.Printf("    headline: %s\n", cycle.Headline)
	}

	if len(cycle.TopAssets) > 0 {
		fmt.Printf("    top assets: %s\n", joinAssets(cycle.TopAssets))
	}
	if len(cycle.Orders) > 0 {
		fmt.Printf("    orders: %s\n", joinOrders(cycle.Orders))
	}

	fmt.Printf("    risk: approved=%d clipped=%d rejected=%d",
		cycle.Risk.Approved, cycle.Risk.Clipped, cycle.Risk.Rejected)
	if len(cycle.Risk.RejectionReasons) > 0 {
		fmt.Printf(" (%s)", strings.Join(cycle.Risk.RejectionReasons, "; "))
	}
	fmt.Println()

	fmt.Printf("    confirmed=%d  ending_nav=%s  positions=%s\n",
		cycle.ConfirmedTrades, deref(cycle.EndingNav), derefInt(cycle.Positions))
}

// joinAssets renders the scored-asset list as "SYM(score), ...", capping the
// number shown so a noisy cycle stays one line.
func joinAssets(assets []guardrail.JournalAsset) string {
	const maxShown = 5
	parts := make([]string, 0, len(assets))
	for i, a := range assets {
		if i >= maxShown {
			parts = append(parts, fmt.Sprintf("(+%d more)", len(assets)-maxShown))
			break
		}
		parts = append(parts, fmt.Sprintf("%s(%s)", a.Symbol, formatFloat(a.Score)))
	}
	return strings.Join(parts, ", ")
}

// joinOrders renders proposed orders as "FROM->TO $amount, ...".
func joinOrders(orders []guardrail.JournalOrder) string {
	parts := make([]string, 0, len(orders))
	for _, o := range orders {
		parts = append(parts, fmt.Sprintf("%s->%s $%s", o.From, o.To, derefFloat(o.AmountUSD)))
	}
	return strings.Join(parts, ", ")
}
