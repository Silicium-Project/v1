// Package auth implements accounts and bearer sessions.
//
// Passwords are hashed with PBKDF2-HMAC-SHA256 (standard library, no dependency) with a
// per-user salt. Session tokens are 256 random bits; only their SHA-256 is stored.
package auth

import (
	"crypto/pbkdf2"
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"net/mail"
	"strings"
	"time"
	"unicode/utf8"

	"sil/api/internal/store"
	"sil/api/internal/wallet"
)

// Errors returned to handlers. They never say whether an e-mail exists.
var (
	ErrInvalidInput       = errors.New("invalid input")
	ErrEmailTaken         = errors.New("e-mail already registered")
	ErrInvalidCredentials = errors.New("invalid credentials")
	ErrUnauthenticated    = errors.New("unauthenticated")
)

const (
	minPassword = 12
	maxPassword = 128
	maxEmail    = 254
	tokenPrefix = "sils_"
)

// Config tunes the service.
type Config struct {
	Iterations int           // PBKDF2 iterations for new passwords
	SessionTTL time.Duration // lifetime of a session
}

// DefaultConfig follows the OWASP 2023 guidance for PBKDF2-HMAC-SHA256.
func DefaultConfig() Config {
	return Config{Iterations: 600_000, SessionTTL: 24 * time.Hour}
}

// Service manages accounts and sessions.
type Service struct {
	store      *store.Store
	cfg        Config
	now        func() time.Time
	dummy      store.User // hashed with a fixed password, to equalise login time for unknown e-mails
	challenges *wallet.Challenges
}

// New builds the service. now may be nil (wall clock).
func New(s *store.Store, cfg Config, now func() time.Time) *Service {
	if now == nil {
		now = time.Now
	}
	svc := &Service{store: s, cfg: cfg, now: now, challenges: wallet.NewChallenges(5*time.Minute, 10_000, now)}
	salt := randomBytes(16)
	hash, _ := hashPassword("dummy-password-for-timing", salt, cfg.Iterations)
	svc.dummy = store.User{Salt: salt, Hash: hash, Iter: cfg.Iterations}
	return svc
}

// NormalizeEmail validates an address and returns its canonical (lower-case) form.
func NormalizeEmail(raw string) (string, error) {
	raw = strings.TrimSpace(raw)
	if raw == "" || len(raw) > maxEmail {
		return "", ErrInvalidInput
	}
	addr, err := mail.ParseAddress(raw)
	if err != nil || addr.Address != raw {
		return "", ErrInvalidInput
	}
	return strings.ToLower(addr.Address), nil
}

func validPassword(password string) bool {
	n := utf8.RuneCountInString(password)
	return n >= minPassword && n <= maxPassword && utf8.ValidString(password)
}

// Register creates an account.
func (s *Service) Register(email, password string) (store.User, error) {
	email, err := NormalizeEmail(email)
	if err != nil || !validPassword(password) {
		return store.User{}, ErrInvalidInput
	}
	salt := randomBytes(16)
	hash, err := hashPassword(password, salt, s.cfg.Iterations)
	if err != nil {
		return store.User{}, err
	}
	user := store.User{
		ID: randomID(), Email: email, Salt: salt, Hash: hash,
		Iter: s.cfg.Iterations, CreatedAt: s.now().UTC(),
	}
	if err := s.store.CreateUser(user); err != nil {
		if errors.Is(err, store.ErrExists) {
			return store.User{}, ErrEmailTaken
		}
		return store.User{}, err
	}
	return user, nil
}

// Login checks the password and opens a session. It returns the clear token once.
func (s *Service) Login(email, password string) (token string, expires time.Time, err error) {
	normalized, normErr := NormalizeEmail(email)
	user, lookupErr := s.store.UserByEmail(normalized)
	if normErr != nil || lookupErr != nil || len(password) > 4*maxPassword {
		// Same work as a real check so timing does not reveal unknown e-mails.
		_, _ = hashPassword(password[:min(len(password), maxPassword)], s.dummy.Salt, s.dummy.Iter)
		return "", time.Time{}, ErrInvalidCredentials
	}
	candidate, err := hashPassword(password, user.Salt, user.Iter)
	if err != nil || subtle.ConstantTimeCompare(candidate, user.Hash) != 1 {
		return "", time.Time{}, ErrInvalidCredentials
	}
	return s.openSession(user.ID)
}

// openSession creates a bearer session for a user and returns the clear token once.
func (s *Service) openSession(userID string) (token string, expires time.Time, err error) {
	raw := randomBytes(32)
	token = tokenPrefix + base64.RawURLEncoding.EncodeToString(raw)
	expires = s.now().Add(s.cfg.SessionTTL).UTC()
	session := store.Session{TokenHash: hashToken(token), UserID: userID, ExpiresAt: expires}
	if err := s.store.PutSession(session, s.now()); err != nil {
		return "", time.Time{}, err
	}
	return token, expires, nil
}

// Authenticate resolves a bearer token to its user id.
func (s *Service) Authenticate(token string) (string, error) {
	if !strings.HasPrefix(token, tokenPrefix) || len(token) > 128 {
		return "", ErrUnauthenticated
	}
	session, err := s.store.SessionByHash(hashToken(token), s.now())
	if err != nil {
		return "", ErrUnauthenticated
	}
	return session.UserID, nil
}

// Logout ends the session of a token.
func (s *Service) Logout(token string) error {
	return s.store.DeleteSession(hashToken(token))
}

func hashPassword(password string, salt []byte, iterations int) ([]byte, error) {
	return pbkdf2.Key(sha256.New, password, salt, iterations, 32)
}

func hashToken(token string) string {
	sum := sha256.Sum256([]byte(token))
	return hex.EncodeToString(sum[:])
}

func randomBytes(n int) []byte {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		panic("system random source failed: " + err.Error())
	}
	return b
}

func randomID() string { return hex.EncodeToString(randomBytes(16)) }

// Profile returns the account of a user.
func (s *Service) Profile(userID string) (store.User, error) {
	return s.store.UserByID(userID)
}

// DeleteAccount erases the account after re-checking the password (a stolen session alone
// must not be enough to destroy an account).
func (s *Service) DeleteAccount(userID, password string) error {
	user, err := s.store.UserByID(userID)
	if err != nil {
		return err
	}
	if !user.HasPassword() {
		return ErrInvalidCredentials
	}
	candidate, err := hashPassword(password[:min(len(password), 4*maxPassword)], user.Salt, user.Iter)
	if err != nil || subtle.ConstantTimeCompare(candidate, user.Hash) != 1 {
		return ErrInvalidCredentials
	}
	return s.store.DeleteUser(userID)
}
