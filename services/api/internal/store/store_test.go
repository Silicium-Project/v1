package store

import (
	"errors"
	"path/filepath"
	"testing"
	"time"
)

var t0 = time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)

func TestEmailIsUnique(t *testing.T) {
	s, _ := Open("")
	if err := s.CreateUser(User{ID: "a", Email: "x@y.z"}); err != nil {
		t.Fatal(err)
	}
	if err := s.CreateUser(User{ID: "b", Email: "x@y.z"}); !errors.Is(err, ErrExists) {
		t.Fatalf("want ErrExists, got %v", err)
	}
}

func TestStateSurvivesARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "sub", "silicium.db")
	s, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	if err := s.CreateUser(User{ID: "a", Email: "x@y.z"}); err != nil {
		t.Fatal(err)
	}
	again, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer again.Close()
	defer s.Close()
	if _, err := again.UserByEmail("x@y.z"); err != nil {
		t.Fatalf("user lost: %v", err)
	}
}

func TestExpiredSessionIsInvisible(t *testing.T) {
	s, _ := Open("")
	_ = s.PutSession(Session{TokenHash: "h", UserID: "a", ExpiresAt: t0.Add(time.Hour)}, t0)
	if _, err := s.SessionByHash("h", t0); err != nil {
		t.Fatal(err)
	}
	if _, err := s.SessionByHash("h", t0.Add(2*time.Hour)); !errors.Is(err, ErrNotFound) {
		t.Fatalf("want ErrNotFound, got %v", err)
	}
}

func TestLinkCodeWorksOnce(t *testing.T) {
	s, _ := Open("")
	_ = s.PutLink(Link{CodeHash: "c", UserID: "u", ExpiresAt: t0.Add(time.Minute)}, t0)
	device := Device{ID: "d1", PublicKey: "k1", LinkedAt: t0}
	if _, err := s.ConsumeLink("c", device, t0); err != nil {
		t.Fatal(err)
	}
	if _, err := s.ConsumeLink("c", Device{ID: "d2", PublicKey: "k2"}, t0); !errors.Is(err, ErrExpired) {
		t.Fatalf("second use: want ErrExpired, got %v", err)
	}
	if got := s.DevicesOf("u"); len(got) != 1 || got[0].UserID != "u" {
		t.Fatalf("devices: %+v", got)
	}
}

func TestLinkCodeExpires(t *testing.T) {
	s, _ := Open("")
	_ = s.PutLink(Link{CodeHash: "c", UserID: "u", ExpiresAt: t0.Add(time.Minute)}, t0)
	_, err := s.ConsumeLink("c", Device{ID: "d", PublicKey: "k"}, t0.Add(time.Hour))
	if !errors.Is(err, ErrExpired) {
		t.Fatalf("want ErrExpired, got %v", err)
	}
}

func TestSameKeyCannotBeLinkedTwice(t *testing.T) {
	s, _ := Open("")
	_ = s.PutLink(Link{CodeHash: "c1", UserID: "u", ExpiresAt: t0.Add(time.Minute)}, t0)
	_ = s.PutLink(Link{CodeHash: "c2", UserID: "v", ExpiresAt: t0.Add(time.Minute)}, t0)
	if _, err := s.ConsumeLink("c1", Device{ID: "d1", PublicKey: "k"}, t0); err != nil {
		t.Fatal(err)
	}
	if _, err := s.ConsumeLink("c2", Device{ID: "d2", PublicKey: "k"}, t0); !errors.Is(err, ErrExists) {
		t.Fatalf("want ErrExists, got %v", err)
	}
}

func TestOnlyTheOwnerCanRevokeADevice(t *testing.T) {
	s, _ := Open("")
	_ = s.PutLink(Link{CodeHash: "c", UserID: "u", ExpiresAt: t0.Add(time.Minute)}, t0)
	_, _ = s.ConsumeLink("c", Device{ID: "d", PublicKey: "k"}, t0)
	if err := s.RevokeDevice("other", "d", t0); !errors.Is(err, ErrNotFound) {
		t.Fatalf("want ErrNotFound, got %v", err)
	}
	if err := s.RevokeDevice("u", "d", t0); err != nil {
		t.Fatal(err)
	}
	if s.DevicesOf("u")[0].RevokedAt == nil {
		t.Fatal("not revoked")
	}
}

func TestJobsAreScopedToTheirOwner(t *testing.T) {
	s, _ := Open("")
	_ = s.AddJob(Job{ID: "j", UserID: "u", CreatedAt: t0})
	if err := s.AddJob(Job{ID: "j", UserID: "u"}); err != nil {
		t.Fatalf("same owner twice: %v", err)
	}
	if err := s.AddJob(Job{ID: "j", UserID: "v"}); !errors.Is(err, ErrExists) {
		t.Fatalf("other owner: %v", err)
	}
	if _, err := s.JobOf("v", "j"); !errors.Is(err, ErrNotFound) {
		t.Fatalf("read by other: %v", err)
	}
}

func TestFailedWriteRollsBackMemory(t *testing.T) {
	s, err := Open(filepath.Join(t.TempDir(), "silicium.db"))
	if err != nil {
		t.Fatal(err)
	}
	// A closed database cannot take the transaction: the write fails.
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	if err := s.CreateUser(User{ID: "a", Email: "x@y.z"}); err == nil {
		t.Fatal("expected a write error")
	}
	if _, err := s.UserByEmail("x@y.z"); !errors.Is(err, ErrNotFound) {
		t.Fatalf("memory ahead of disk: %v", err)
	}
}
