// Command grctl is a read-only operator CLI for the Guardrail API.
//
// Usage:
//
//	grctl [--api URL] [--json] <command> [args]
//
// Commands: status regime portfolio trades risk signals proof verify events
// cockpit watch [seconds]. The API base URL comes from --api or $GUARDRAIL_API
// (default http://127.0.0.1:8080).
package main

import (
	"fmt"
	"os"
	"strings"

	"guardrail/grctl/internal/client"
	"guardrail/grctl/internal/commands"
)

func main() {
	api := os.Getenv("GUARDRAIL_API")
	if api == "" {
		api = "http://127.0.0.1:8080"
	}
	jsonOut := false

	// Parse global flags from anywhere in the args; the first bare token is the
	// command, the rest are its args.
	var command string
	var rest []string
	for _, a := range os.Args[1:] {
		switch {
		case a == "--json":
			jsonOut = true
		case a == "--api":
			// handled in the value-bearing form below; bare --api is ignored
		case strings.HasPrefix(a, "--api="):
			api = strings.TrimPrefix(a, "--api=")
		case strings.HasPrefix(a, "--"):
			// unknown flag — ignore for forward-compat
		case command == "":
			command = a
		default:
			rest = append(rest, a)
		}
	}

	if command == "" || command == "help" {
		fmt.Printf("grctl — read-only Guardrail operator CLI\n\n")
		fmt.Printf("usage: grctl [--api URL] [--json] <command> [args]\n")
		fmt.Printf("commands: %s\n", strings.Join(commands.Names(), " "))
		fmt.Printf("api: %s (override with --api=URL or $GUARDRAIL_API)\n", api)
		if command == "" {
			os.Exit(2)
		}
		return
	}

	c := client.New(api)
	if err := commands.Run(c, command, jsonOut, rest); err != nil {
		fmt.Fprintf(os.Stderr, "error: %v\n", err)
		os.Exit(1)
	}
}
