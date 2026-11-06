package httpx

import (
	"sync"
	"time"
)

// Limiter is a token-bucket limiter keyed by string (client address, e-mail...). The table
// is bounded: when full, idle entries are dropped first, so an attacker cannot grow memory
// by varying the key.
type Limiter struct {
	mu      sync.Mutex
	rate    float64 // tokens per second
	burst   float64
	maxKeys int
	buckets map[string]*bucket
}

type bucket struct {
	tokens float64
	last   time.Time
}

// NewLimiter allows `burst` events at once and `perSecond` afterwards, per key.
func NewLimiter(perSecond float64, burst int, maxKeys int) *Limiter {
	return &Limiter{rate: perSecond, burst: float64(burst), maxKeys: maxKeys, buckets: map[string]*bucket{}}
}

// Allow consumes one token for key at time now.
func (l *Limiter) Allow(key string, now time.Time) bool {
	l.mu.Lock()
	defer l.mu.Unlock()
	b, ok := l.buckets[key]
	if !ok {
		if len(l.buckets) >= l.maxKeys {
			l.evict(now)
		}
		b = &bucket{tokens: l.burst, last: now}
		l.buckets[key] = b
	}
	if elapsed := now.Sub(b.last).Seconds(); elapsed > 0 {
		b.tokens = min(l.burst, b.tokens+elapsed*l.rate)
		b.last = now
	}
	if b.tokens < 1 {
		return false
	}
	b.tokens--
	return true
}

// evict removes buckets that have refilled completely, or the oldest one if none has.
func (l *Limiter) evict(now time.Time) {
	var oldestKey string
	var oldest time.Time
	for key, b := range l.buckets {
		if b.tokens+now.Sub(b.last).Seconds()*l.rate >= l.burst {
			delete(l.buckets, key)
			continue
		}
		if oldestKey == "" || b.last.Before(oldest) {
			oldestKey, oldest = key, b.last
		}
	}
	if len(l.buckets) >= l.maxKeys && oldestKey != "" {
		delete(l.buckets, oldestKey)
	}
}
