// Package guardrail is a dependency-free, idiomatic Go client for the
// Guardrail Alpha read/control HTTP API.
//
// It uses only the standard library (net/http, encoding/json, context) and
// mirrors the route set and JSON shapes exposed by the Guardrail API and its
// sibling TypeScript and Python SDKs. The API is read-only: this client never
// mutates agent state.
//
// # Construction
//
// Create a client with NewClient and optional functional options:
//
//	c := guardrail.NewClient("http://localhost:8080",
//		guardrail.WithTimeout(5*time.Second),
//	)
//
// # Calling endpoints
//
// Every method takes a context.Context as its first argument so callers can
// apply deadlines and cancellation:
//
//	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
//	defer cancel()
//	health, err := c.Health(ctx)
//
// # Errors
//
// On a non-2xx response the client returns an *APIError carrying the HTTP
// status code and the raw response body. Transport errors (for example a
// refused connection) are returned as ordinary wrapped errors.
//
// # Typed vs dynamic payloads
//
// Headline routes (compete readiness, history points, regime, alerts,
// backtest, walk-forward, sweep, health) return first-class structs.
// Endpoints whose payloads are dynamic are modelled loosely as map[string]any
// so the SDK stays forward-compatible with backend changes.
package guardrail
