package wallet

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"sync"
	"time"
)

// Purpose says what a signed challenge authorises. A login signature can never be replayed
// as an account deletion, and the reverse.
type Purpose string

const (
	PurposeLogin  Purpose = "login"
	PurposeDelete Purpose = "delete"
)

var (
	// ErrChallenge: unknown, used, expired, or issued for another address or purpose.
	ErrChallenge = errors.New("challenge unknown, expired or already used")
	// ErrFull: too many open challenges; the caller should retry later.
	ErrFull = errors.New("too many open challenges")
)

type challenge struct {
	address [32]byte
	purpose Purpose
	expires time.Time
}

// Challenges issues single-use nonces. It lives in memory: a restart invalidates them,
// which is safe (the user simply asks again).
type Challenges struct {
	mu    sync.Mutex
	items map[string]challenge
	ttl   time.Duration
	max   int
	now   func() time.Time
}

// NewChallenges builds a store. now may be nil (wall clock).
func NewChallenges(ttl time.Duration, max int, now func() time.Time) *Challenges {
	if now == nil {
		now = time.Now
	}
	return &Challenges{items: map[string]challenge{}, ttl: ttl, max: max, now: now}
}

// Message is the exact text the wallet shows and signs. It binds the purpose, the address
// and the nonce, so a signature cannot be moved to another context.
func Message(purpose Purpose, address, nonce string) string {
	return fmt.Sprintf("Silicium %s\nAddress: %s\nNonce: %s", purpose, address, nonce)
}

// Issue creates a nonce for address and returns the message to sign.
func (c *Challenges) Issue(address [32]byte, purpose Purpose) (nonce, message string, expires time.Time, err error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	now := c.now()
	c.sweep(now)
	if len(c.items) >= c.max {
		return "", "", time.Time{}, ErrFull
	}
	var raw [16]byte
	if _, err := rand.Read(raw[:]); err != nil {
		return "", "", time.Time{}, err
	}
	nonce = hex.EncodeToString(raw[:])
	expires = now.Add(c.ttl)
	c.items[nonce] = challenge{address: address, purpose: purpose, expires: expires}
	return nonce, Message(purpose, EncodeBase58(address[:]), nonce), expires, nil
}

// Consume checks and removes a nonce: it works once, for one address and one purpose.
// It returns the message that was issued.
func (c *Challenges) Consume(nonce string, address [32]byte, purpose Purpose) (string, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	item, ok := c.items[nonce]
	if !ok {
		return "", ErrChallenge
	}
	delete(c.items, nonce) // single use, even when the checks below fail
	if !c.now().Before(item.expires) || item.address != address || item.purpose != purpose {
		return "", ErrChallenge
	}
	return Message(purpose, EncodeBase58(address[:]), nonce), nil
}

func (c *Challenges) sweep(now time.Time) {
	for nonce, item := range c.items {
		if !now.Before(item.expires) {
			delete(c.items, nonce)
		}
	}
}

// Verify checks an Ed25519 signature made by address over the message bytes (UTF-8).
func Verify(address [32]byte, message string, signature []byte) bool {
	if len(signature) != ed25519.SignatureSize {
		return false
	}
	return ed25519.Verify(address[:], []byte(message), signature)
}
