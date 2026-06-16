// Package commands implements the grctl subcommands over the read-only API.
package commands

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"text/tabwriter"
	"time"

	"guardrail/grctl/internal/client"
)

// route maps each subcommand to the API path it reads.
var route = map[string]string{
	"status":    "/health",
	"regime":    "/regime",
	"portfolio": "/portfolio",
	"trades":    "/trades",
	"risk":      "/risk",
	"signals":   "/signals",
	"proof":     "/proof",
	"verify":    "/proof/verify",
	"events":    "/events",
	"cockpit":   "/cockpit",
}

// Names returns the sorted list of supported subcommands (plus `watch`).
func Names() []string {
	out := make([]string, 0, len(route)+1)
	for k := range route {
		out = append(out, k)
	}
	out = append(out, "watch")
	sort.Strings(out)
	return out
}

func printJSON(v any) {
	b, _ := json.MarshalIndent(v, "", "  ")
	fmt.Println(string(b))
}

func kv(w *tabwriter.Writer, k string, v any) {
	fmt.Fprintf(w, "%s\t%v\n", k, v)
}

// Run dispatches a subcommand. `jsonOut` forces raw JSON output.
func Run(c *client.Client, name string, jsonOut bool, args []string) error {
	if name == "watch" {
		return watch(c, args)
	}
	path, ok := route[name]
	if !ok {
		return fmt.Errorf("unknown command %q (try: %v)", name, Names())
	}
	data, err := c.GetMap(path)
	if err != nil {
		return err
	}
	if jsonOut {
		printJSON(data)
		return nil
	}
	render(name, data)
	return nil
}

// render prints a compact, human-friendly summary per command; unknown shapes
// fall back to pretty JSON so nothing is hidden.
func render(name string, data map[string]any) {
	w := tabwriter.NewWriter(os.Stdout, 0, 2, 2, ' ', 0)
	defer w.Flush()
	switch name {
	case "status":
		kv(w, "status", data["status"])
		kv(w, "events", data["events"])
	case "regime":
		kv(w, "regime", data["regime"])
		kv(w, "exposure", data["exposure"])
	case "verify":
		kv(w, "passed", data["passed"])
		if checks, ok := data["checks"].([]any); ok {
			kv(w, "checks", len(checks))
		}
	case "events":
		if events, ok := data["events"].([]any); ok {
			kv(w, "events", len(events))
			for i, e := range events {
				if i >= 10 {
					break
				}
				if m, ok := e.(map[string]any); ok {
					fmt.Fprintf(w, "%v\t%v\n", m["timestamp"], m["event_type"])
				}
			}
		}
	default:
		// Portfolio/trades/risk/proof/signals/cockpit: show top-level keys.
		keys := make([]string, 0, len(data))
		for k := range data {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			kv(w, k, summarize(data[k]))
		}
	}
}

// summarize renders a value compactly (lengths for arrays/objects).
func summarize(v any) string {
	switch t := v.(type) {
	case []any:
		return fmt.Sprintf("[%d items]", len(t))
	case map[string]any:
		return fmt.Sprintf("{%d fields}", len(t))
	default:
		return fmt.Sprintf("%v", v)
	}
}

// watch polls /regime on an interval (default 5s, or args[0] seconds).
func watch(c *client.Client, args []string) error {
	interval := 5 * time.Second
	if len(args) > 0 {
		if n, err := time.ParseDuration(args[0] + "s"); err == nil {
			interval = n
		}
	}
	fmt.Printf("watching /regime every %s (ctrl-c to stop)\n", interval)
	for {
		data, err := c.GetMap("/regime")
		if err != nil {
			fmt.Printf("%s  error: %v\n", time.Now().Format(time.RFC3339), err)
		} else {
			fmt.Printf("%s  regime=%v exposure=%v\n",
				time.Now().Format(time.RFC3339), data["regime"], data["exposure"])
		}
		time.Sleep(interval)
	}
}
