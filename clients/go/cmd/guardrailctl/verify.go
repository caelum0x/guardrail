package main

import (
	"flag"
	"fmt"
	"os"
	"text/tabwriter"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// cmdVerify fetches /proof/verify and prints the per-check pass/fail table that
// the agent computes against its on-disk policy + run report. It is offline-safe
// and always returns exitOK.
func cmdVerify(argv []string) int {
	fs := flag.NewFlagSet("verify", flag.ContinueOnError)
	common := registerCommon(fs)
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl verify [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	ctx, cancel := callContext()
	defer cancel()

	resp, err := common.newClient().ProofVerify(ctx)
	if err != nil {
		fmt.Println(unavailable("verify", err))
		return exitOK
	}

	if common.json {
		printJSON(resp)
		return exitOK
	}

	renderVerify(resp)
	return exitOK
}

// renderVerify prints the verification summary line and per-check table.
func renderVerify(resp *guardrail.ProofVerifyResponse) {
	passed, failed := resp.Counts()
	overall := "FAIL"
	if resp.Passed {
		overall = "PASS"
	}
	fmt.Printf("proof verification: %s  (%d passed, %d failed)\n", overall, passed, failed)
	if resp.ReportPath != "" {
		fmt.Printf("report: %s\n", resp.ReportPath)
	}
	if resp.Reason != "" {
		fmt.Printf("reason: %s\n", resp.Reason)
	}

	if len(resp.Checks) == 0 {
		fmt.Println("\n(no checks reported)")
		return
	}

	fmt.Println()
	tw := tabwriter.NewWriter(os.Stdout, 0, 4, 2, ' ', 0)
	fmt.Fprintln(tw, "STATUS\tCHECK\tDETAIL")
	for _, c := range resp.Checks {
		fmt.Fprintf(tw, "%s\t%s\t%s\n", verifyStatusLabel(c), c.Name, c.Detail)
	}
	_ = tw.Flush()

	if len(resp.RecomputedPolicyHashes) > 0 {
		fmt.Println("\nrecomputed policy hashes:")
		for _, h := range resp.RecomputedPolicyHashes {
			fmt.Printf("  %s  %s\n", h.SHA256, h.File)
		}
	}
}

// verifyStatusLabel renders a check's status as an upper-case PASS/FAIL marker.
func verifyStatusLabel(c guardrail.ServerCheck) string {
	if c.Passed() {
		return "PASS"
	}
	return "FAIL"
}
