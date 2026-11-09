package network

import (
	"context"
	"sync"
	"time"
)

// Cache keeps the last summary for a short time, so that a page refreshing every few seconds
// does not ask the node on every visit.
type Cache struct {
	mu     sync.Mutex
	ttl    time.Duration
	now    func() time.Time
	at     time.Time
	status Status
	filled bool
}

// NewCache builds a cache for ttl. now may be nil (wall clock).
func NewCache(ttl time.Duration, now func() time.Time) *Cache {
	if now == nil {
		now = time.Now
	}
	return &Cache{ttl: ttl, now: now}
}

// Get returns the summary of the network, asking src again only when the cached one is stale.
// A node that cannot be asked gives an unreachable answer, which is not cached.
func (c *Cache) Get(ctx context.Context, src Source) Status {
	c.mu.Lock()
	defer c.mu.Unlock()
	now := c.now()
	if c.filled && now.Sub(c.at) < c.ttl {
		return c.status
	}
	if src == nil {
		return Unreachable(now)
	}
	peers, errPeers := src.Peers(ctx)
	report, errReport := src.Worker(ctx)
	if errPeers != nil || errReport != nil {
		return Unreachable(now)
	}
	status, err := Summarize(peers, report, now)
	if err != nil {
		return Unreachable(now)
	}
	c.status, c.at, c.filled = status, now, true
	return status
}
