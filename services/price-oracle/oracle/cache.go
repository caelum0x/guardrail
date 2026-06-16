package oracle

import (
	"context"
	"sync"
	"time"
)

// Snapshot is the cache's view of the tracked universe at a point in time.
type Snapshot struct {
	Prices    map[string]float64 `json:"prices"`
	UpdatedAt time.Time          `json:"updated_at"`
	AgeSeconds float64           `json:"age_seconds"`
	Stale     bool               `json:"stale"`
	Count     int                `json:"count"`
}

// Cache holds the most recent prices behind a TTL, refreshing from CoinGecko
// on demand. Reads are cheap (RWMutex); a stale read triggers a refresh. If a
// refresh fails but we hold previous prices, the stale snapshot is served with
// Stale=true rather than failing the request.
type Cache struct {
	client  *CoinGeckoClient
	ttl     time.Duration
	mu      sync.RWMutex
	prices  map[string]float64
	updated time.Time
}

// NewCache builds a cache over the given client with the given freshness TTL.
func NewCache(client *CoinGeckoClient, ttl time.Duration) *Cache {
	if ttl <= 0 {
		ttl = 30 * time.Second
	}
	return &Cache{client: client, ttl: ttl, prices: map[string]float64{}}
}

func (c *Cache) snapshot() Snapshot {
	c.mu.RLock()
	defer c.mu.RUnlock()
	prices := make(map[string]float64, len(c.prices))
	for k, v := range c.prices {
		prices[k] = v
	}
	age := time.Since(c.updated)
	return Snapshot{
		Prices:     prices,
		UpdatedAt:  c.updated,
		AgeSeconds: age.Seconds(),
		Stale:      age >= c.ttl,
		Count:      len(prices),
	}
}

func (c *Cache) fresh() bool {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return len(c.prices) > 0 && time.Since(c.updated) < c.ttl
}

// Get returns a fresh snapshot, refreshing from CoinGecko if the cache is stale.
func (c *Cache) Get(ctx context.Context) (Snapshot, error) {
	if c.fresh() {
		return c.snapshot(), nil
	}
	return c.Refresh(ctx)
}

// Refresh forces a fetch from CoinGecko and updates the cache. On fetch error,
// any previously-cached prices are returned (marked Stale) alongside the error.
func (c *Cache) Refresh(ctx context.Context) (Snapshot, error) {
	prices, err := c.client.FetchPrices(ctx)
	if err != nil {
		c.mu.RLock()
		have := len(c.prices) > 0
		c.mu.RUnlock()
		if have {
			s := c.snapshot()
			s.Stale = true
			return s, err
		}
		return Snapshot{Prices: map[string]float64{}}, err
	}
	c.mu.Lock()
	c.prices = prices
	c.updated = time.Now()
	c.mu.Unlock()
	return c.snapshot(), nil
}

// Price returns the cached price for one symbol, refreshing if stale.
func (c *Cache) Price(ctx context.Context, symbol string) (float64, bool, error) {
	snap, err := c.Get(ctx)
	if err != nil && snap.Count == 0 {
		return 0, false, err
	}
	v, ok := snap.Prices[symbol]
	return v, ok, nil
}
