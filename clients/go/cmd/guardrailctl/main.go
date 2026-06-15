// Command guardrailctl is a small operator CLI for the Guardrail Alpha API.
//
// It is offline-safe by design: every subcommand prints a useful line and
// exits 0 even when the API is unreachable (for example a refused connection),
// so it is harmless to run in CI or against a stopped backend. The --base flag
// selects the API address (default http://localhost:8080) and --json switches
// sensible commands to machine-readable output.
//
// Subcommands:
//
//	watch     poll /compete + /regime on an interval and print a refreshing
//	          one-line status; --once for a single tick, Ctrl-C to stop.
//	ensemble  print the current regime and the per-skill weight table.
//	journal   print a compact per-cycle decision journal.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"os"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// exitOK and exitUsage are the only process exit codes. Operational failures
// (API down, decode errors) deliberately still exit 0 so the tool is safe to
// run offline; only a usage mistake exits non-zero.
const (
	exitOK    = 0
	exitUsage = 2
)

// requestTimeout bounds each individual HTTP call so an unreachable host fails
// fast rather than hanging on the client's longer overall timeout.
const requestTimeout = 5 * time.Second

func main() {
	os.Exit(run(os.Args[1:]))
}

// run dispatches argv (excluding the program name) to a subcommand and returns
// the process exit code. It is a function rather than inline in main so the
// dispatch is unit-testable.
func run(argv []string) int {
	if len(argv) == 0 {
		usage(os.Stderr)
		return exitUsage
	}

	cmd, rest := argv[0], argv[1:]
	switch cmd {
	case "watch":
		return cmdWatch(rest)
	case "ensemble":
		return cmdEnsemble(rest)
	case "journal":
		return cmdJournal(rest)
	case "help", "-h", "--help":
		usage(os.Stdout)
		return exitOK
	default:
		fmt.Fprintf(os.Stderr, "guardrailctl: unknown command %q\n\n", cmd)
		usage(os.Stderr)
		return exitUsage
	}
}

// commonFlags holds the flags shared by every subcommand. registerCommon wires
// them onto a FlagSet so each command parses --base/--json identically.
type commonFlags struct {
	base string
	json bool
}

func registerCommon(fs *flag.FlagSet) *commonFlags {
	c := &commonFlags{}
	fs.StringVar(&c.base, "base", guardrail.DefaultBaseURL, "Guardrail API base URL")
	fs.BoolVar(&c.json, "json", false, "emit machine-readable JSON instead of a table")
	return c
}

// newClient builds an SDK client for the resolved base URL with a short
// per-call timeout. Per-call context deadlines still apply on top of this.
func (c *commonFlags) newClient() *guardrail.Client {
	return guardrail.NewClient(c.base, guardrail.WithTimeout(requestTimeout))
}

// callContext returns a context bounded by requestTimeout for a single API
// call, plus its cancel func which the caller must defer.
func callContext() (context.Context, context.CancelFunc) {
	return context.WithTimeout(context.Background(), requestTimeout)
}

// unavailable formats a one-line, non-fatal notice for a failed API call,
// distinguishing an API-level error (status + body) from a transport failure
// such as a refused connection.
func unavailable(label string, err error) string {
	var apiErr *guardrail.APIError
	if errors.As(err, &apiErr) {
		return fmt.Sprintf("%s: API error %d: %s", label, apiErr.Status, apiErr.Body)
	}
	return fmt.Sprintf("%s: unavailable: %v", label, err)
}

// usage prints the top-level help text to w.
func usage(w *os.File) {
	fmt.Fprint(w, `guardrailctl — operator CLI for the Guardrail Alpha API

Usage:
  guardrailctl <command> [flags]

Commands:
  watch      poll /compete + /regime and print a refreshing status line
  ensemble   show the current regime and per-skill ensemble weights
  journal    show a compact per-cycle decision journal
  help       show this help

Common flags:
  --base string   API base URL (default `+guardrail.DefaultBaseURL+`)
  --json          emit JSON instead of a table (where supported)

All commands are offline-safe: they print a notice and exit 0 when the API is
unreachable. Run "guardrailctl <command> -h" for command-specific flags.
`)
}
