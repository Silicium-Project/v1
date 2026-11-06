// Package devices links a machine (an Ed25519 key) to an account.
//
// Flow: the logged-in user asks for a one-time code (10 minutes). The machine signs
// "code || public key" under the domain "sil/v1/device-link" and sends the proof. The API
// checks the signature, consumes the code and records the key. See contracts/device-link.md
// for the byte layout and a test vector shared with the Rust side.
package devices

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"strings"
	"time"
	"unicode/utf8"

	"sil/api/internal/store"
)

// Domain is the signature domain tag; it matches sil-crypto Domain::DeviceLink.
const Domain = "sil/v1/device-link"

const (
	codeLength    = 12
	codeAlphabet  = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789" // 32 symbols, no 0/O/1/I
	maxLabelRunes = 64
	// MaxDevices is the number of non-revoked devices one account may hold.
	MaxDevices = 20
)

// CodeTTL is how long a link code stays valid.
const CodeTTL = 10 * time.Minute

var (
	ErrInvalidProof = errors.New("invalid link proof")
	ErrBadCode      = errors.New("link code unknown, expired or already used")
	ErrKeyInUse     = errors.New("this key is already linked")
	ErrTooMany      = errors.New("too many devices")
	ErrInvalidInput = errors.New("invalid input")
)

// Service manages device links.
type Service struct {
	store *store.Store
	now   func() time.Time
}

// New builds the service. now may be nil (wall clock).
func New(s *store.Store, now func() time.Time) *Service {
	if now == nil {
		now = time.Now
	}
	return &Service{store: s, now: now}
}

// NewCode creates a one-time link code for the user. The clear code is returned once.
func (s *Service) NewCode(userID string) (code string, expires time.Time, err error) {
	active := 0
	for _, d := range s.store.DevicesOf(userID) {
		if d.RevokedAt == nil {
			active++
		}
	}
	if active >= MaxDevices {
		return "", time.Time{}, ErrTooMany
	}
	code = randomCode()
	expires = s.now().Add(CodeTTL).UTC()
	link := store.Link{CodeHash: hashCode(code), UserID: userID, ExpiresAt: expires}
	if err := s.store.PutLink(link, s.now()); err != nil {
		return "", time.Time{}, err
	}
	return code, expires, nil
}

// Message returns the bytes the device must sign (before domain framing): the code as
// typed (upper case) followed by the 32 raw public key bytes.
func Message(code string, publicKey []byte) []byte {
	return append([]byte(strings.ToUpper(code)), publicKey...)
}

// Frame returns tag || 0x00 || message, the exact bytes signed (sil-crypto Domain::frame).
func Frame(message []byte) []byte {
	framed := append([]byte(Domain), 0)
	return append(framed, message...)
}

// Link verifies the proof and registers the device on the account that issued the code.
func (s *Service) Link(code, publicKeyHex, signatureHex, label string) (store.Device, error) {
	code = strings.ToUpper(strings.TrimSpace(code))
	if len(code) != codeLength || utf8.RuneCountInString(label) > maxLabelRunes {
		return store.Device{}, ErrInvalidInput
	}
	key, err := hex.DecodeString(publicKeyHex)
	if err != nil || len(key) != ed25519.PublicKeySize || publicKeyHex != strings.ToLower(publicKeyHex) {
		return store.Device{}, ErrInvalidInput
	}
	signature, err := hex.DecodeString(signatureHex)
	if err != nil || len(signature) != ed25519.SignatureSize {
		return store.Device{}, ErrInvalidInput
	}
	if !ed25519.Verify(key, Frame(Message(code, key)), signature) {
		return store.Device{}, ErrInvalidProof
	}
	device := store.Device{
		ID: randomHex(16), PublicKey: publicKeyHex, Label: strings.TrimSpace(label),
		LinkedAt: s.now().UTC(),
	}
	stored, err := s.store.ConsumeLink(hashCode(code), device, s.now())
	switch {
	case errors.Is(err, store.ErrExpired):
		return store.Device{}, ErrBadCode
	case errors.Is(err, store.ErrExists):
		return store.Device{}, ErrKeyInUse
	case err != nil:
		return store.Device{}, err
	}
	return stored, nil
}

// List returns the devices of a user.
func (s *Service) List(userID string) []store.Device { return s.store.DevicesOf(userID) }

// Revoke marks a device as revoked.
func (s *Service) Revoke(userID, deviceID string) error {
	err := s.store.RevokeDevice(userID, deviceID, s.now())
	if errors.Is(err, store.ErrNotFound) {
		return store.ErrNotFound
	}
	return err
}

func hashCode(code string) string {
	sum := sha256.Sum256([]byte(code))
	return hex.EncodeToString(sum[:])
}

func randomCode() string {
	raw := make([]byte, codeLength)
	if _, err := rand.Read(raw); err != nil {
		panic("system random source failed: " + err.Error())
	}
	out := make([]byte, codeLength)
	for i, b := range raw {
		out[i] = codeAlphabet[int(b)%len(codeAlphabet)] // 256 % 32 == 0: no modulo bias
	}
	return string(out)
}

func randomHex(n int) string {
	raw := make([]byte, n)
	if _, err := rand.Read(raw); err != nil {
		panic("system random source failed: " + err.Error())
	}
	return hex.EncodeToString(raw)
}
