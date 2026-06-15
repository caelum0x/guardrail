package main

import (
	"flag"
	"fmt"
	"os"
	"strings"
	"text/tabwriter"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// cmdSkills fetches /skills (the Track-2 Skill catalog) and prints id, name, and
// regimes. With an optional positional id argument it fetches /skills/{id} and
// prints the detail instead. It is offline-safe and always returns exitOK.
func cmdSkills(argv []string) int {
	fs := flag.NewFlagSet("skills", flag.ContinueOnError)
	common := registerCommon(fs)
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl skills [ID] [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	// A single positional argument selects the per-skill detail route.
	if id := fs.Arg(0); id != "" {
		return skillDetail(common, id)
	}
	return skillCatalog(common)
}

// skillCatalog fetches and renders the full /skills catalog.
func skillCatalog(common *commonFlags) int {
	ctx, cancel := callContext()
	defer cancel()

	resp, err := common.newClient().Skills(ctx)
	if err != nil {
		fmt.Println(unavailable("skills", err))
		return exitOK
	}

	if common.json {
		printJSON(resp)
		return exitOK
	}

	renderSkillCatalog(resp)
	return exitOK
}

// skillDetail fetches and renders /skills/{id}.
func skillDetail(common *commonFlags, id string) int {
	ctx, cancel := callContext()
	defer cancel()

	resp, err := common.newClient().SkillByID(ctx, id)
	if err != nil {
		fmt.Println(unavailable("skills", err))
		return exitOK
	}

	if common.json {
		printJSON(resp)
		return exitOK
	}

	renderSkillDetail(id, resp)
	return exitOK
}

// renderSkillCatalog prints the catalog as an id / name / regimes table.
func renderSkillCatalog(resp *guardrail.SkillsResponse) {
	fmt.Printf("skill catalog: %d skill(s)  (%s)\n", resp.Count, orDash(resp.IndexPath))
	if len(resp.Skills) == 0 {
		fmt.Println("\n(no skills published — empty or unavailable index)")
		return
	}

	fmt.Println()
	tw := tabwriter.NewWriter(os.Stdout, 0, 4, 2, ' ', 0)
	fmt.Fprintln(tw, "ID\tNAME\tREGIMES")
	for _, s := range resp.Skills {
		fmt.Fprintf(tw, "%s\t%s\t%s\n", s.ID, orDash(s.Name), joinOrDash(s.Regimes))
	}
	_ = tw.Flush()
}

// renderSkillDetail prints the per-skill detail block. The id argument is the
// requested id, used in the not-found message when the response carries an error.
func renderSkillDetail(id string, resp *guardrail.SkillDetail) {
	if resp.Error != "" {
		fmt.Printf("skill %q: %s\n", id, resp.Error)
		return
	}

	fmt.Printf("%s  (%s)\n", orDash(resp.Name), orDash(resp.ID))
	if strings.TrimSpace(resp.Summary) != "" {
		fmt.Printf("  summary: %s\n", resp.Summary)
	}
	if strings.TrimSpace(resp.Description) != "" {
		fmt.Printf("  description: %s\n", resp.Description)
	}
	fmt.Printf("  regimes: %s\n", joinOrDash(resp.Regimes))
	if len(resp.Inputs) > 0 {
		fmt.Printf("  inputs: %s\n", strings.Join(resp.Inputs, ", "))
	}
	fmt.Printf("  eligible universe: %d  examples: %d (on disk: %d)\n",
		resp.EligibleUniverseSize, resp.ExamplesCount, resp.ExamplesOnDisk)
	if strings.TrimSpace(resp.SpecFile) != "" {
		fmt.Printf("  spec file: %s\n", resp.SpecFile)
	}
	if len(resp.SpecSections) > 0 {
		fmt.Printf("  spec sections: %s\n", strings.Join(resp.SpecSections, ", "))
	}
}

// joinOrDash renders a string slice as a comma-separated list, or "-" when empty.
func joinOrDash(values []string) string {
	if len(values) == 0 {
		return "-"
	}
	return strings.Join(values, ", ")
}
