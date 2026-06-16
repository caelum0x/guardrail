// Package relay implements a concurrent webhook fan-out relay.
//
// An incoming alert (arbitrary JSON) is delivered concurrently to every
// configured webhook target. Each delivery is bounded by a per-target timeout
// and retried with backoff on transient failures. The result is a per-target
// delivery report.
package relay

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

// Defaults applied when configuration values are zero/unset.
const (
	DefaultTimeout     = 5 * time.Second
	DefaultRetries     = 2
	DefaultBackoff     = 250 * time.Millisecond
	maxResponseSnippet = 512
	maxRetries         = 10
	maxTimeout         = 60 * time.Second
)

// Config controls relay delivery behavior.
type Config struct {
	// Targets is the list of webhook URLs to fan out to.
	Targets []string
	// Timeout bounds a single delivery attempt to one target.
	Timeout time.Duration
	// Retries is the number of additional attempts after the first failure.
	Retries int
	// Backoff is the base wait between attempts; it grows linearly per attempt.
	Backoff time.Duration
}

// normalized returns a copy of the config with defaults and bounds applied.
func (c Config) normalized() Config {
	out := Config{
		Targets: c.Targets,
		Timeout: c.Timeout,
		Retries: c.Retries,
		Backoff: c.Backoff,
	}
	if out.Timeout <= 0 {
		out.Timeout = DefaultTimeout
	}
	if out.Timeout > maxTimeout {
		out.Timeout = maxTimeout
	}
	if out.Retries < 0 {
		out.Retries = DefaultRetries
	}
	if out.Retries > maxRetries {
		out.Retries = maxRetries
	}
	if out.Backoff <= 0 {
		out.Backoff = DefaultBackoff
	}
	return out
}

// TargetResult is the delivery outcome for a single webhook target.
type TargetResult struct {
	Target     string `json:"target"`
	Success    bool   `json:"success"`
	StatusCode int    `json:"status_code,omitempty"`
	Attempts   int    `json:"attempts"`
	DurationMS int64  `json:"duration_ms"`
	Error      string `json:"error,omitempty"`
	Response   string `json:"response,omitempty"`
}

// Report aggregates per-target results for one fan-out operation.
type Report struct {
	Total     int            `json:"total"`
	Delivered int            `json:"delivered"`
	Failed    int            `json:"failed"`
	DurationMS int64         `json:"duration_ms"`
	Results   []TargetResult `json:"results"`
}

// Relay delivers alert payloads to configured webhook targets.
type Relay struct {
	cfg    Config
	client *http.Client
}

// ParseTargets splits a comma-separated list of URLs, trimming blanks and
// validating that each entry is an absolute http(s) URL.
func ParseTargets(csv string) ([]string, error) {
	var targets []string
	for _, raw := range strings.Split(csv, ",") {
		t := strings.TrimSpace(raw)
		if t == "" {
			continue
		}
		if !strings.HasPrefix(t, "http://") && !strings.HasPrefix(t, "https://") {
			return nil, fmt.Errorf("invalid webhook target %q: must start with http:// or https://", t)
		}
		targets = append(targets, t)
	}
	return targets, nil
}

// New builds a Relay from cfg, applying defaults. It returns an error if no
// targets are configured.
func New(cfg Config) (*Relay, error) {
	norm := cfg.normalized()
	if len(norm.Targets) == 0 {
		return nil, errors.New("relay: no webhook targets configured")
	}
	return &Relay{
		cfg: norm,
		// The per-attempt context deadline bounds each request; the client
		// timeout is a hard upper bound covering the whole attempt including
		// body read.
		client: &http.Client{Timeout: norm.Timeout},
	}, nil
}

// Targets returns the configured target URLs.
func (r *Relay) Targets() []string {
	out := make([]string, len(r.cfg.Targets))
	copy(out, r.cfg.Targets)
	return out
}

// Fanout delivers payload concurrently to all configured targets and returns a
// per-target report. The provided context bounds the whole operation; per-target
// timeouts still apply within it.
func (r *Relay) Fanout(ctx context.Context, payload []byte) Report {
	start := time.Now()
	results := make([]TargetResult, len(r.cfg.Targets))

	var wg sync.WaitGroup
	for i, target := range r.cfg.Targets {
		wg.Add(1)
		go func(idx int, url string) {
			defer wg.Done()
			results[idx] = r.deliver(ctx, url, payload)
		}(i, target)
	}
	wg.Wait()

	report := Report{
		Total:      len(results),
		Results:    results,
		DurationMS: time.Since(start).Milliseconds(),
	}
	for _, res := range results {
		if res.Success {
			report.Delivered++
		} else {
			report.Failed++
		}
	}
	return report
}

// deliver attempts delivery to a single target with retry + linear backoff.
func (r *Relay) deliver(ctx context.Context, url string, payload []byte) TargetResult {
	start := time.Now()
	res := TargetResult{Target: url}

	totalAttempts := r.cfg.Retries + 1
	for attempt := 1; attempt <= totalAttempts; attempt++ {
		res.Attempts = attempt

		status, body, err := r.attempt(ctx, url, payload)
		res.StatusCode = status
		if err == nil && status >= 200 && status < 300 {
			res.Success = true
			res.Response = snippet(body)
			res.Error = ""
			res.DurationMS = time.Since(start).Milliseconds()
			return res
		}

		if err != nil {
			res.Error = err.Error()
		} else {
			res.Error = fmt.Sprintf("non-2xx status %d", status)
			res.Response = snippet(body)
		}

		// Do not retry if the overall context is done or this was the last try.
		if attempt == totalAttempts || ctx.Err() != nil {
			break
		}

		// Linear backoff, interruptible by context cancellation.
		wait := time.Duration(attempt) * r.cfg.Backoff
		timer := time.NewTimer(wait)
		select {
		case <-ctx.Done():
			timer.Stop()
			res.Error = fmt.Sprintf("context canceled during backoff: %v", ctx.Err())
			res.DurationMS = time.Since(start).Milliseconds()
			return res
		case <-timer.C:
		}
	}

	res.DurationMS = time.Since(start).Milliseconds()
	return res
}

// attempt performs a single bounded HTTP POST to url.
func (r *Relay) attempt(ctx context.Context, url string, payload []byte) (int, []byte, error) {
	attemptCtx, cancel := context.WithTimeout(ctx, r.cfg.Timeout)
	defer cancel()

	req, err := http.NewRequestWithContext(attemptCtx, http.MethodPost, url, bytes.NewReader(payload))
	if err != nil {
		return 0, nil, fmt.Errorf("build request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("User-Agent", "guardrail-notification-relay/1.0")

	resp, err := r.client.Do(req)
	if err != nil {
		return 0, nil, fmt.Errorf("deliver: %w", err)
	}
	defer resp.Body.Close()

	// Read a bounded amount of the response so a hostile/large target cannot
	// exhaust memory; the rest is discarded to allow connection reuse.
	body, _ := io.ReadAll(io.LimitReader(resp.Body, maxResponseSnippet))
	_, _ = io.Copy(io.Discard, resp.Body)
	return resp.StatusCode, body, nil
}

// snippet trims a response body to a printable, bounded string.
func snippet(body []byte) string {
	s := strings.TrimSpace(string(body))
	if len(s) > maxResponseSnippet {
		s = s[:maxResponseSnippet]
	}
	return s
}
