// Package client is a tiny read-only HTTP client for the Guardrail API.
package client

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Client targets a Guardrail read-only API base URL.
type Client struct {
	BaseURL string
	http    *http.Client
}

// New builds a client with a bounded timeout; the base URL's trailing slash is trimmed.
func New(baseURL string) *Client {
	return &Client{
		BaseURL: strings.TrimRight(baseURL, "/"),
		http:    &http.Client{Timeout: 10 * time.Second},
	}
}

// GetRaw fetches a path and returns the raw response body, erroring on non-2xx.
func (c *Client) GetRaw(path string) ([]byte, error) {
	resp, err := c.http.Get(c.BaseURL + path)
	if err != nil {
		return nil, fmt.Errorf("GET %s: %w", path, err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("read %s: %w", path, err)
	}
	if resp.StatusCode/100 != 2 {
		return nil, fmt.Errorf("GET %s: status %d", path, resp.StatusCode)
	}
	return body, nil
}

// GetMap fetches a path and decodes it into a generic JSON object.
func (c *Client) GetMap(path string) (map[string]any, error) {
	body, err := c.GetRaw(path)
	if err != nil {
		return nil, err
	}
	var out map[string]any
	if err := json.Unmarshal(body, &out); err != nil {
		return nil, fmt.Errorf("decode %s: %w", path, err)
	}
	return out, nil
}
