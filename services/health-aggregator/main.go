// Command health-aggregator polls the /health endpoints of the Guardrail
// services concurrently and serves a single aggregated status. Read-only ops
// tooling: it only issues GETs and never touches the trading loop.
//
// Targets come from the HEALTH_TARGETS env var as a comma-separated list of
// name=url pairs, e.g.:
//
//	HEALTH_TARGETS="api=http://127.0.0.1:8080/health,oracle=http://127.0.0.1:8090/health"
//
// Endpoints:
//
//	GET /health   aggregate status (200 if all up, 503 if any down)
//	GET /targets  the configured target list
package main

import (
	"context"
	"encoding/json"
	"log"
	"net/http"
	"os"
	"sort"
	"strings"
	"sync"
	"time"
)

type target struct {
	Name string
	URL  string
}

type result struct {
	Name       string `json:"name"`
	URL        string `json:"url"`
	Up         bool   `json:"up"`
	StatusCode int    `json:"status_code,omitempty"`
	LatencyMS  int64  `json:"latency_ms"`
	Error      string `json:"error,omitempty"`
}

type aggregate struct {
	Status  string   `json:"status"` // "ok" | "degraded"
	Up      int      `json:"up"`
	Down    int      `json:"down"`
	Total   int      `json:"total"`
	Checked string   `json:"checked_at"`
	Results []result `json:"results"`
}

func parseTargets(raw string) []target {
	var out []target
	for _, part := range strings.Split(raw, ",") {
		part = strings.TrimSpace(part)
		if part == "" {
			continue
		}
		name, url, ok := strings.Cut(part, "=")
		if !ok {
			// Bare URL: derive a name from it.
			name, url = part, part
		}
		out = append(out, target{Name: strings.TrimSpace(name), URL: strings.TrimSpace(url)})
	}
	return out
}

func defaultTargets() []target {
	return []target{
		{Name: "api", URL: "http://127.0.0.1:8080/health"},
		{Name: "price-oracle", URL: "http://127.0.0.1:8090/health"},
		{Name: "exporter", URL: "http://127.0.0.1:9100/healthz"},
	}
}

func check(ctx context.Context, client *http.Client, t target) result {
	start := time.Now()
	r := result{Name: t.Name, URL: t.URL}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, t.URL, nil)
	if err != nil {
		r.Error = err.Error()
		return r
	}
	resp, err := client.Do(req)
	r.LatencyMS = time.Since(start).Milliseconds()
	if err != nil {
		r.Error = err.Error()
		return r
	}
	defer resp.Body.Close()
	r.StatusCode = resp.StatusCode
	r.Up = resp.StatusCode/100 == 2
	if !r.Up {
		r.Error = "non-2xx status"
	}
	return r
}

func aggregateHealth(ctx context.Context, client *http.Client, targets []target) aggregate {
	results := make([]result, len(targets))
	var wg sync.WaitGroup
	for i, t := range targets {
		wg.Add(1)
		go func(i int, t target) {
			defer wg.Done()
			results[i] = check(ctx, client, t)
		}(i, t)
	}
	wg.Wait()
	sort.Slice(results, func(i, j int) bool { return results[i].Name < results[j].Name })

	up := 0
	for _, r := range results {
		if r.Up {
			up++
		}
	}
	status := "ok"
	if up < len(results) {
		status = "degraded"
	}
	return aggregate{
		Status:  status,
		Up:      up,
		Down:    len(results) - up,
		Total:   len(results),
		Checked: time.Now().UTC().Format(time.RFC3339),
		Results: results,
	}
}

func main() {
	targets := defaultTargets()
	if env := os.Getenv("HEALTH_TARGETS"); env != "" {
		targets = parseTargets(env)
	}
	addr := os.Getenv("HEALTH_ADDR")
	if addr == "" {
		addr = ":8095"
	}
	client := &http.Client{Timeout: 5 * time.Second}

	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", func(w http.ResponseWriter, r *http.Request) {
		ctx, cancel := context.WithTimeout(r.Context(), 8*time.Second)
		defer cancel()
		agg := aggregateHealth(ctx, client, targets)
		w.Header().Set("Content-Type", "application/json")
		if agg.Status != "ok" {
			w.WriteHeader(http.StatusServiceUnavailable)
		}
		_ = json.NewEncoder(w).Encode(agg)
	})
	mux.HandleFunc("GET /targets", func(w http.ResponseWriter, _ *http.Request) {
		names := make([]string, len(targets))
		for i, t := range targets {
			names[i] = t.Name + "=" + t.URL
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]any{"targets": names})
	})

	log.Printf("health-aggregator listening on %s (%d targets)", addr, len(targets))
	srv := &http.Server{Addr: addr, Handler: mux, ReadHeaderTimeout: 5 * time.Second}
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("server error: %v", err)
	}
}
