package relay

import (
	"encoding/json"
	"io"
	"net/http"
	"time"
)

// maxRequestBody bounds the size of an incoming alert payload (1 MiB).
const maxRequestBody = 1 << 20

// healthResponse is the GET /health body.
type healthResponse struct {
	Status  string   `json:"status"`
	Targets int      `json:"targets"`
	URLs    []string `json:"urls"`
	Time    string   `json:"time"`
}

// errorResponse is returned for client/server errors.
type errorResponse struct {
	Error string `json:"error"`
}

// Server is an HTTP handler exposing the relay over POST /notify and GET /health.
type Server struct {
	relay *Relay
	mux   *http.ServeMux
}

// NewServer wires routes for the given relay.
func NewServer(r *Relay) *Server {
	s := &Server{relay: r, mux: http.NewServeMux()}
	s.mux.HandleFunc("/notify", s.handleNotify)
	s.mux.HandleFunc("/health", s.handleHealth)
	return s
}

// ServeHTTP implements http.Handler.
func (s *Server) ServeHTTP(w http.ResponseWriter, req *http.Request) {
	s.mux.ServeHTTP(w, req)
}

func (s *Server) handleHealth(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodGet {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed; use GET")
		return
	}
	urls := s.relay.Targets()
	writeJSON(w, http.StatusOK, healthResponse{
		Status:  "ok",
		Targets: len(urls),
		URLs:    urls,
		Time:    time.Now().UTC().Format(time.RFC3339),
	})
}

func (s *Server) handleNotify(w http.ResponseWriter, req *http.Request) {
	if req.Method != http.MethodPost {
		writeError(w, http.StatusMethodNotAllowed, "method not allowed; use POST")
		return
	}

	body, err := io.ReadAll(io.LimitReader(req.Body, maxRequestBody+1))
	if err != nil {
		writeError(w, http.StatusBadRequest, "failed to read request body")
		return
	}
	if len(body) == 0 {
		writeError(w, http.StatusBadRequest, "empty request body; expected a JSON alert")
		return
	}
	if len(body) > maxRequestBody {
		writeError(w, http.StatusRequestEntityTooLarge, "request body exceeds 1 MiB limit")
		return
	}

	// Validate that the payload is well-formed JSON before fanning out, so we
	// never forward garbage to downstream webhooks.
	if !json.Valid(body) {
		writeError(w, http.StatusBadRequest, "request body is not valid JSON")
		return
	}

	report := s.relay.Fanout(req.Context(), body)

	// 207 Multi-Status when some (but not all) targets failed; 502 when all
	// failed; 200 when everything succeeded.
	status := http.StatusOK
	switch {
	case report.Delivered == 0:
		status = http.StatusBadGateway
	case report.Failed > 0:
		status = http.StatusMultiStatus
	}
	writeJSON(w, status, report)
}

func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	enc := json.NewEncoder(w)
	enc.SetEscapeHTML(false)
	_ = enc.Encode(v)
}

func writeError(w http.ResponseWriter, status int, msg string) {
	writeJSON(w, status, errorResponse{Error: msg})
}
