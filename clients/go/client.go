package guardrail

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"time"
)

// DefaultBaseURL is the address the API listens on in local development.
const DefaultBaseURL = "http://localhost:8080"

// defaultTimeout is applied to the internal *http.Client when the caller does
// not supply one. Per-call deadlines should be set via context.
const defaultTimeout = 10 * time.Second

// Client is a read-only client for the Guardrail Alpha API. It is safe for
// concurrent use by multiple goroutines.
type Client struct {
	baseURL string
	http    *http.Client
}

// Option customizes a Client during construction.
type Option func(*Client)

// WithHTTPClient supplies a custom *http.Client (for proxies, transports, or
// tests). A nil client is ignored.
func WithHTTPClient(hc *http.Client) Option {
	return func(c *Client) {
		if hc != nil {
			c.http = hc
		}
	}
}

// WithTimeout sets the request timeout on the client's underlying
// *http.Client. Non-positive values are ignored. Note that a context deadline
// passed to an individual call takes effect independently of this timeout.
func WithTimeout(d time.Duration) Option {
	return func(c *Client) {
		if d > 0 {
			c.http.Timeout = d
		}
	}
}

// NewClient builds a Client targeting baseURL. When baseURL is empty,
// DefaultBaseURL is used. Trailing slashes are trimmed so route joins are
// predictable.
func NewClient(baseURL string, opts ...Option) *Client {
	if baseURL == "" {
		baseURL = DefaultBaseURL
	}
	c := &Client{
		baseURL: strings.TrimRight(baseURL, "/"),
		http:    &http.Client{Timeout: defaultTimeout},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// BaseURL returns the normalized base URL the client targets.
func (c *Client) BaseURL() string {
	return c.baseURL
}

// APIError is returned when the API responds with a non-2xx status. It carries
// the HTTP status code and the raw response body for diagnostics.
type APIError struct {
	// Status is the HTTP status code returned by the API.
	Status int
	// Path is the request path that produced the error.
	Path string
	// Body is the raw response body, truncated for safety.
	Body string
}

// Error implements the error interface.
func (e *APIError) Error() string {
	if e.Body == "" {
		return fmt.Sprintf("guardrail: GET %s failed: status %d", e.Path, e.Status)
	}
	return fmt.Sprintf("guardrail: GET %s failed: status %d: %s", e.Path, e.Status, e.Body)
}

// maxErrorBody caps how much of an error response body is retained.
const maxErrorBody = 4 << 10

// maxRawBody caps how much of a raw JSON response body is read, bounding memory
// use when decoding untrusted payloads.
const maxRawBody = 1 << 20

// do builds and executes a GET-style request against path, applying query
// parameters, decoding a JSON response into out, and translating non-2xx
// responses into *APIError. When out is nil the body is drained and discarded.
//
// body is reserved for future write endpoints; when non-nil it is JSON encoded
// and sent with method (defaulting to GET when method is empty).
func (c *Client) do(ctx context.Context, method, path string, query url.Values, body, out any) error {
	if method == "" {
		method = http.MethodGet
	}

	full := c.baseURL + path
	if len(query) > 0 {
		full += "?" + query.Encode()
	}

	var reader io.Reader
	if body != nil {
		encoded, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("guardrail: encode request body for %s: %w", path, err)
		}
		reader = strings.NewReader(string(encoded))
	}

	req, err := http.NewRequestWithContext(ctx, method, full, reader)
	if err != nil {
		return fmt.Errorf("guardrail: build request for %s: %w", path, err)
	}
	req.Header.Set("Accept", "application/json")
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}

	resp, err := c.http.Do(req)
	if err != nil {
		return fmt.Errorf("guardrail: request %s: %w", path, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		raw, _ := io.ReadAll(io.LimitReader(resp.Body, maxErrorBody))
		return &APIError{
			Status: resp.StatusCode,
			Path:   path,
			Body:   strings.TrimSpace(string(raw)),
		}
	}

	if out == nil {
		_, _ = io.Copy(io.Discard, resp.Body)
		return nil
	}

	if err := json.NewDecoder(resp.Body).Decode(out); err != nil {
		return fmt.Errorf("guardrail: decode response for %s: %w", path, err)
	}
	return nil
}

// doText executes a GET against path and returns the raw response body as a
// string, used for the text/plain endpoints (metrics, markdown report,
// submission export).
func (c *Client) doText(ctx context.Context, path string, query url.Values) (string, error) {
	full := c.baseURL + path
	if len(query) > 0 {
		full += "?" + query.Encode()
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, full, nil)
	if err != nil {
		return "", fmt.Errorf("guardrail: build request for %s: %w", path, err)
	}
	req.Header.Set("Accept", "text/plain")

	resp, err := c.http.Do(req)
	if err != nil {
		return "", fmt.Errorf("guardrail: request %s: %w", path, err)
	}
	defer resp.Body.Close()

	raw, readErr := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body := raw
		if len(body) > maxErrorBody {
			body = body[:maxErrorBody]
		}
		return "", &APIError{
			Status: resp.StatusCode,
			Path:   path,
			Body:   strings.TrimSpace(string(body)),
		}
	}
	if readErr != nil {
		return "", fmt.Errorf("guardrail: read response for %s: %w", path, readErr)
	}
	return string(raw), nil
}

// doRaw executes a GET against path and returns the raw JSON response body,
// used by callers that decode the body themselves (for example the proof
// verifier, which needs byte-exact access to numeric fields).
func (c *Client) doRaw(ctx context.Context, path string) ([]byte, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.baseURL+path, nil)
	if err != nil {
		return nil, fmt.Errorf("guardrail: build request for %s: %w", path, err)
	}
	req.Header.Set("Accept", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, fmt.Errorf("guardrail: request %s: %w", path, err)
	}
	defer resp.Body.Close()

	raw, readErr := io.ReadAll(io.LimitReader(resp.Body, maxRawBody))
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, &APIError{
			Status: resp.StatusCode,
			Path:   path,
			Body:   strings.TrimSpace(string(raw)),
		}
	}
	if readErr != nil {
		return nil, fmt.Errorf("guardrail: read response for %s: %w", path, readErr)
	}
	return raw, nil
}
