// Command price-oracle is a small read-only microservice that serves live USD
// prices for the Guardrail BSC universe, sourced from the free CoinGecko public
// API and cached behind a TTL. No API key, no chain access, no writes.
package main

import (
	"log"
	"net/http"
	"os"
	"time"

	"guardrail/price-oracle/oracle"
)

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8090"
	}

	ttl := durationEnv("ORACLE_TTL", 30*time.Second)
	timeout := durationEnv("ORACLE_HTTP_TIMEOUT", 10*time.Second)

	client := oracle.NewCoinGeckoClient(timeout)
	cache := oracle.NewCache(client, ttl)
	srv := oracle.NewServer(cache)

	addr := ":" + port
	server := &http.Server{
		Addr:              addr,
		Handler:           srv.Routes(),
		ReadHeaderTimeout: 5 * time.Second,
	}

	log.Printf("price-oracle listening on %s (ttl=%s, tracking %d symbols)", addr, ttl, len(oracle.Universe))
	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("server error: %v", err)
	}
}

// durationEnv reads a duration from an env var (Go duration string, e.g. "45s"),
// falling back to def on absence or parse error.
func durationEnv(key string, def time.Duration) time.Duration {
	if v := os.Getenv(key); v != "" {
		if d, err := time.ParseDuration(v); err == nil {
			return d
		}
	}
	return def
}
