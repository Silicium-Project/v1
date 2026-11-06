package auth

import (
	"errors"
	"strings"
	"testing"
	"time"

	"sil/api/internal/store"
)

func newService(t *testing.T) (*Service, *time.Time) {
	t.Helper()
	s, _ := store.Open("")
	clock := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	svc := New(s, Config{Iterations: 1000, SessionTTL: time.Hour}, func() time.Time { return clock })
	return svc, &clock
}

const goodPassword = "correct horse battery"

func TestRegisterThenLogin(t *testing.T) {
	svc, _ := newService(t)
	if _, err := svc.Register("Alice@Example.com", goodPassword); err != nil {
		t.Fatal(err)
	}
	token, _, err := svc.Login("alice@example.com", goodPassword)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Authenticate(token); err != nil {
		t.Fatalf("session refused: %v", err)
	}
}

func TestEmailsAreCaseInsensitiveAndUnique(t *testing.T) {
	svc, _ := newService(t)
	_, _ = svc.Register("a@b.co", goodPassword)
	if _, err := svc.Register("A@B.CO", goodPassword); !errors.Is(err, ErrEmailTaken) {
		t.Fatalf("want ErrEmailTaken, got %v", err)
	}
}

func TestWeakOrMalformedInputIsRefused(t *testing.T) {
	svc, _ := newService(t)
	for _, c := range []struct{ email, password string }{
		{"a@b.co", "short"},
		{"a@b.co", strings.Repeat("x", maxPassword+1)},
		{"not an email", goodPassword},
		{"Bob <b@b.co>", goodPassword},
		{"", goodPassword},
	} {
		if _, err := svc.Register(c.email, c.password); !errors.Is(err, ErrInvalidInput) {
			t.Errorf("%q: want ErrInvalidInput, got %v", c.email, err)
		}
	}
}

func TestWrongPasswordAndUnknownEmailLookAlike(t *testing.T) {
	svc, _ := newService(t)
	_, _ = svc.Register("a@b.co", goodPassword)
	_, _, e1 := svc.Login("a@b.co", "wrong password!!")
	_, _, e2 := svc.Login("nobody@b.co", goodPassword)
	if !errors.Is(e1, ErrInvalidCredentials) || !errors.Is(e2, ErrInvalidCredentials) {
		t.Fatalf("got %v and %v", e1, e2)
	}
}

func TestSessionExpiresAndLogoutEndsIt(t *testing.T) {
	svc, clock := newService(t)
	_, _ = svc.Register("a@b.co", goodPassword)
	token, _, _ := svc.Login("a@b.co", goodPassword)
	if err := svc.Logout(token); err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Authenticate(token); !errors.Is(err, ErrUnauthenticated) {
		t.Fatalf("after logout: %v", err)
	}
	token, _, _ = svc.Login("a@b.co", goodPassword)
	*clock = clock.Add(2 * time.Hour)
	if _, err := svc.Authenticate(token); !errors.Is(err, ErrUnauthenticated) {
		t.Fatalf("after expiry: %v", err)
	}
}

func TestGarbageTokensAreRefused(t *testing.T) {
	svc, _ := newService(t)
	for _, token := range []string{"", "x", "sils_", "sils_" + strings.Repeat("a", 500)} {
		if _, err := svc.Authenticate(token); !errors.Is(err, ErrUnauthenticated) {
			t.Errorf("%q accepted", token)
		}
	}
}

func TestStoredHashIsNotThePassword(t *testing.T) {
	svc, _ := newService(t)
	user, _ := svc.Register("a@b.co", goodPassword)
	if strings.Contains(string(user.Hash), goodPassword) || len(user.Salt) != 16 {
		t.Fatal("password stored in clear or salt missing")
	}
}
