package oracle

import (
	"encoding/json"
	"log"
	"net/http"
	"strings"
	"time"
)

// Server wires the cache to HTTP handlers.
type Server struct {
	cache *Cache
}

// NewServer builds a server over the given cache.
func NewServer(cache *Cache) *Server {
	return &Server{cache: cache}
}

// Routes returns the configured HTTP handler (Go 1.22+ method+path patterns).
func (s *Server) Routes() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", s.handleHealth)
	mux.HandleFunc("GET /prices", s.handlePrices)
	mux.HandleFunc("GET /prices/refresh", s.handleRefresh)
	mux.HandleFunc("GET /prices/{symbol}", s.handleSymbol)
	return logging(mux)
}

func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}

func (s *Server) handleHealth(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, http.StatusOK, map[string]any{
		"status":  "ok",
		"service": "price-oracle",
		"tracked": Symbols(),
	})
}

func (s *Server) handlePrices(w http.ResponseWriter, r *http.Request) {
	snap, err := s.cache.Get(r.Context())
	if err != nil && snap.Count == 0 {
		writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, snap)
}

func (s *Server) handleRefresh(w http.ResponseWriter, r *http.Request) {
	snap, err := s.cache.Refresh(r.Context())
	if err != nil && snap.Count == 0 {
		writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
		return
	}
	writeJSON(w, http.StatusOK, snap)
}

func (s *Server) handleSymbol(w http.ResponseWriter, r *http.Request) {
	symbol := strings.ToUpper(r.PathValue("symbol"))
	price, ok, err := s.cache.Price(r.Context(), symbol)
	if err != nil {
		writeJSON(w, http.StatusBadGateway, map[string]string{"error": err.Error()})
		return
	}
	if !ok {
		writeJSON(w, http.StatusNotFound, map[string]any{
			"error":   "symbol not tracked or unavailable",
			"symbol":  symbol,
			"tracked": Symbols(),
		})
		return
	}
	writeJSON(w, http.StatusOK, PriceEntry{Symbol: symbol, USD: price})
}

// PriceEntry is a single symbol/price pair.
type PriceEntry struct {
	Symbol string  `json:"symbol"`
	USD    float64 `json:"usd"`
}

// logging wraps a handler with a one-line access log.
func logging(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()
		next.ServeHTTP(w, r)
		log.Printf("%s %s %s", r.Method, r.URL.Path, time.Since(start))
	})
}
