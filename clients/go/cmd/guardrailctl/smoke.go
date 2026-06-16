package main

import (
	"flag"
	"fmt"
	"os"
	"text/tabwriter"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// smokeCheck pairs a quant endpoint's display name with the SDK call that
// exercises it. The call takes its own bounded context so one slow endpoint
// cannot stall the rest.
type smokeCheck struct {
	name string
	run  func(*guardrail.Client) (map[string]any, error)
}

// smokeChecks mirrors scripts/smoke_quant.sh and the TS CLI `smoke`: the same
// nine read-only quant endpoints, with inputs that produce a real (non-error)
// response. This is the Go-native sibling of those.
func smokeChecks() []smokeCheck {
	return []smokeCheck{
		{"ta", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.TA(ctx, "rsi", []float64{44, 44.3, 44.1, 43.6, 44.3, 44.8}, 5, 0)
		}},
		{"fees", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.Fees(ctx, map[string]string{"notional_usd": "25000", "quantity": "12", "side": "buy"})
		}},
		{"sizer", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.Sizer(ctx, "kelly", map[string]string{"win_prob": "0.6", "odds": "1.5"})
		}},
		{"orderbook", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.Orderbook(ctx, "s,limit,101,5;b,market,,6")
		}},
		{"pnl", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.PnL(ctx, "CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3")
		}},
		{"correlation", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.Correlation(ctx, "BTC:0.01,-0.02,0.03;ETH:0.012,-0.018,0.025")
		}},
		{"equity/indicators", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.EquityIndicators(ctx, "rsi", 14)
		}},
		{"portfolio/risk", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.PortfolioRisk(ctx)
		}},
		{"cmc/capabilities", func(c *guardrail.Client) (map[string]any, error) {
			ctx, cancel := callContext()
			defer cancel()
			return c.CMCCapabilities(ctx)
		}},
	}
}

// smokeResult is one endpoint's classified outcome.
type smokeResult struct {
	Name    string `json:"name"`
	Outcome string `json:"outcome"` // pass | warn | fail
	Detail  string `json:"detail,omitempty"`
}

// classifySmoke maps a result to an outcome: a transport/decode error is FAIL,
// an "error" field in the body is WARN (reachable but needs a prior run),
// otherwise PASS.
func classifySmoke(body map[string]any, err error) (string, string) {
	if err != nil {
		return "fail", err.Error()
	}
	if msg, ok := body["error"]; ok {
		return "warn", fmt.Sprintf("%v", msg)
	}
	return "pass", ""
}

// cmdSmoke exercises every quant endpoint through the SDK and prints a
// PASS/WARN/FAIL table. Unlike every other subcommand it is a gate: it returns
// exitSmokeFail when any endpoint fails to respond.
func cmdSmoke(argv []string) int {
	fs := flag.NewFlagSet("smoke", flag.ContinueOnError)
	common := registerCommon(fs)
	fs.Usage = func() {
		fmt.Fprintln(os.Stderr, "Usage: guardrailctl smoke [--base URL] [--json]")
		fs.PrintDefaults()
	}
	if err := fs.Parse(argv); err != nil {
		return exitUsage
	}

	client := common.newClient()
	checks := smokeChecks()
	results := make([]smokeResult, 0, len(checks))
	fails := 0
	for _, chk := range checks {
		body, err := chk.run(client)
		outcome, detail := classifySmoke(body, err)
		if outcome == "fail" {
			fails++
		}
		results = append(results, smokeResult{Name: chk.name, Outcome: outcome, Detail: detail})
	}

	if common.json {
		printJSON(map[string]any{"base": common.base, "fails": fails, "results": results})
		return smokeExit(fails)
	}

	fmt.Printf("quant API smoke against %s\n", common.base)
	tw := tabwriter.NewWriter(os.Stdout, 0, 4, 2, ' ', 0)
	for _, r := range results {
		detail := ""
		if r.Detail != "" {
			detail = "(" + r.Detail + ")"
		}
		fmt.Fprintf(tw, "  [%s]\t%s\t%s\n", smokeLabel(r.Outcome), r.Name, detail)
	}
	_ = tw.Flush()
	fmt.Println()
	if fails == 0 {
		fmt.Println("OK — all quant endpoints responded with valid JSON")
	} else {
		fmt.Printf("FAILED — %d endpoint(s) did not respond correctly\n", fails)
	}
	return smokeExit(fails)
}

// smokeLabel renders an outcome as an upper-case fixed-width marker.
func smokeLabel(outcome string) string {
	switch outcome {
	case "pass":
		return "PASS"
	case "warn":
		return "WARN"
	default:
		return "FAIL"
	}
}

// smokeExit maps the failure count to a process exit code.
func smokeExit(fails int) int {
	if fails == 0 {
		return exitOK
	}
	return exitSmokeFail
}
