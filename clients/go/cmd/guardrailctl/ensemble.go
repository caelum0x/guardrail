package main

import (
	"flag"
	"fmt"
	"os"
	"text/tabwriter"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// cmdEnsemble fetches /ensemble and prints the current regime plus the
// per-skill weight table. It is offline-safe and always returns exitOK.
func cmdEnsemble(argv []string) int {
	fs := flag.NewFlagSet("ensemble", flag.ContinueOnError)
	common := registerCommon(fs)
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl ensemble [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	ctx, cancel := callContext()
	defer cancel()

	resp, err := common.newClient().Ensemble(ctx)
	if err != nil {
		// Offline: report on stdout and exit 0 so scripts can rely on it.
		fmt.Println(unavailable("ensemble", err))
		return exitOK
	}

	if common.json {
		printJSON(resp)
		return exitOK
	}

	renderEnsembleTable(resp)
	return exitOK
}

// renderEnsembleTable prints the human-readable ensemble view: a header with
// the current regime, then a matrix of regimes (rows) by skills (columns) with
// the active regime marked.
func renderEnsembleTable(resp *guardrail.EnsembleResponse) {
	fmt.Printf("%s v%s  reserve=%s  max_risk=%s%%\n",
		resp.Name, resp.Version, resp.ReserveSymbol, formatFloat(resp.MaxRiskAllocationPct))
	fmt.Printf("current regime: %s\n\n", deref(resp.CurrentRegime))

	if len(resp.Skills) == 0 || len(resp.Regimes) == 0 {
		fmt.Println("(no ensemble weights reported)")
		return
	}

	tw := tabwriter.NewWriter(os.Stdout, 0, 4, 2, ' ', 0)
	// Header row: regime label + one column per skill id.
	fmt.Fprint(tw, "REGIME")
	for _, skill := range resp.Skills {
		fmt.Fprintf(tw, "\t%s", skill.ID)
	}
	fmt.Fprintln(tw)

	current := deref(resp.CurrentRegime)
	for _, row := range resp.Regimes {
		label := row.Regime
		if label == current {
			label = "* " + label
		}
		fmt.Fprint(tw, label)
		for _, skill := range resp.Skills {
			fmt.Fprintf(tw, "\t%s", formatFloat(row.Weights[skill.ID]))
		}
		fmt.Fprintln(tw)
	}
	_ = tw.Flush()

	fmt.Println()
	fmt.Println("skills:")
	for _, skill := range resp.Skills {
		fmt.Printf("  %-28s %s\n", skill.ID, skill.Label)
	}
	if len(resp.ActiveWeights) > 0 {
		fmt.Println("(* marks the currently active regime row)")
	}
}
