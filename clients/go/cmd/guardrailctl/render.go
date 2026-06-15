package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"
)

// printJSON marshals v as indented JSON to stdout. Encoding failures are
// reported to stderr but never abort the program, preserving offline-safety.
func printJSON(v any) {
	encoded, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		fmt.Fprintf(os.Stderr, "guardrailctl: encode output: %v\n", err)
		return
	}
	fmt.Println(string(encoded))
}

// formatFloat renders a weight/score compactly, trimming trailing zeros.
func formatFloat(f float64) string {
	return strconv.FormatFloat(f, 'f', -1, 64)
}

// deref returns the pointed-to string or "-" when nil/empty.
func deref(s *string) string {
	if s == nil {
		return "-"
	}
	return orDash(*s)
}

// derefInt returns the pointed-to int as text or "-" when nil.
func derefInt(i *int) string {
	if i == nil {
		return "-"
	}
	return strconv.Itoa(*i)
}

// derefFloat returns the pointed-to float as text or "-" when nil.
func derefFloat(f *float64) string {
	if f == nil {
		return "-"
	}
	return formatFloat(*f)
}
