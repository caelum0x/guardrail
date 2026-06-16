package oracle

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"sort"
	"strings"
	"time"
)

// Universe maps a tracked trading symbol to its CoinGecko coin id.
// These are the BSC-universe symbols the oracle tracks.
var Universe = map[string]string{
	"BNB":  "binancecoin",
	"CAKE": "pancakeswap-token",
	"ETH":  "ethereum",
	"BTC":  "bitcoin",
	"USDT": "tether",
	"XRP":  "ripple",
	"ADA":  "cardano",
	"DOGE": "dogecoin",
	"AVAX": "avalanche-2",
	"LINK": "chainlink",
}

// idToSymbol is the reverse lookup of Universe, built once at init.
var idToSymbol = func() map[string]string {
	m := make(map[string]string, len(Universe))
	for sym, id := range Universe {
		m[id] = sym
	}
	return m
}()

// Symbols returns the sorted list of tracked symbols.
func Symbols() []string {
	out := make([]string, 0, len(Universe))
	for sym := range Universe {
		out = append(out, sym)
	}
	sort.Strings(out)
	return out
}

// coinIDs returns the sorted list of CoinGecko ids for the tracked universe.
func coinIDs() []string {
	out := make([]string, 0, len(Universe))
	for _, id := range Universe {
		out = append(out, id)
	}
	sort.Strings(out)
	return out
}

// CoinGeckoClient is a thin real HTTP client for the CoinGecko public API.
type CoinGeckoClient struct {
	baseURL string
	http    *http.Client
}

// NewCoinGeckoClient builds a client with a bounded request timeout.
func NewCoinGeckoClient(timeout time.Duration) *CoinGeckoClient {
	if timeout <= 0 {
		timeout = 10 * time.Second
	}
	return &CoinGeckoClient{
		baseURL: "https://api.coingecko.com/api/v3",
		http: &http.Client{
			Timeout: timeout,
		},
	}
}

// simplePriceResponse models the CoinGecko /simple/price JSON shape:
// { "bitcoin": { "usd": 12345.67 }, ... }
type simplePriceResponse map[string]map[string]float64

// FetchPrices fetches live USD prices for the entire tracked universe.
// It returns a map keyed by trading symbol (e.g. "BTC") to USD price.
func (c *CoinGeckoClient) FetchPrices(ctx context.Context) (map[string]float64, error) {
	ids := coinIDs()
	if len(ids) == 0 {
		return map[string]float64{}, nil
	}

	q := url.Values{}
	q.Set("ids", strings.Join(ids, ","))
	q.Set("vs_currencies", "usd")
	endpoint := fmt.Sprintf("%s/simple/price?%s", c.baseURL, q.Encode())

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, fmt.Errorf("build request: %w", err)
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("User-Agent", "guardrail-price-oracle/1.0")

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, fmt.Errorf("coingecko request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("coingecko returned status %d", resp.StatusCode)
	}

	var decoded simplePriceResponse
	if err := json.NewDecoder(resp.Body).Decode(&decoded); err != nil {
		return nil, fmt.Errorf("decode coingecko response: %w", err)
	}

	prices := make(map[string]float64, len(decoded))
	for id, quote := range decoded {
		sym, ok := idToSymbol[id]
		if !ok {
			continue
		}
		usd, ok := quote["usd"]
		if !ok {
			continue
		}
		prices[sym] = usd
	}

	if len(prices) == 0 {
		return nil, fmt.Errorf("coingecko returned no usable prices")
	}

	return prices, nil
}
