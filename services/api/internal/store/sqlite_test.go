package store

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// Changes made through the API must survive a restart, including deletions and telemetry.
func TestChangesSurviveARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "silicium.db")
	s, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	if err := s.CreateUser(User{ID: "u", Email: "x@y.z", CreatedAt: t0}); err != nil {
		t.Fatal(err)
	}
	_ = s.PutLink(Link{CodeHash: "c", UserID: "u", ExpiresAt: t0.Add(time.Minute)}, t0)
	if _, err := s.ConsumeLink("c", Device{ID: "d", PublicKey: "k", Label: "box", LinkedAt: t0}, t0); err != nil {
		t.Fatal(err)
	}
	score := 0.75
	if err := s.SetTelemetry("k", Telemetry{ObservedAt: t0, ReceivedAt: t0, CPUCores: 8, ReputationScore: &score}); err != nil {
		t.Fatal(err)
	}
	if err := s.AddJob(Job{ID: "j", UserID: "u", Label: "sha", CreatedAt: t0}); err != nil {
		t.Fatal(err)
	}
	if err := s.DeleteUser("u"); err != nil {
		t.Fatal(err)
	}
	if err := s.CreateUser(User{ID: "v", Address: "wallet", CreatedAt: t0}); err != nil {
		t.Fatal(err)
	}
	if err := s.Close(); err != nil {
		t.Fatal(err)
	}

	again, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer again.Close()
	if _, err := again.UserByEmail("x@y.z"); err == nil {
		t.Fatal("deleted user came back")
	}
	if got := again.DevicesOf("u"); len(got) != 0 {
		t.Fatalf("deleted devices came back: %+v", got)
	}
	if _, err := again.UserByAddress("wallet"); err != nil {
		t.Fatalf("wallet user lost: %v", err)
	}
	if err := again.AddJob(Job{ID: "j", UserID: "v"}); err != nil {
		t.Fatalf("job of deleted user still indexed: %v", err)
	}
}

// The former store.json is imported once, renamed, and never read again.
func TestLegacyJSONIsImportedOnce(t *testing.T) {
	dir := t.TempDir()
	legacy := snapshot{
		Users: map[string]User{"a": {ID: "a", Email: "old@example.com", CreatedAt: t0}},
		Jobs:  map[string]Job{"j": {ID: "j", UserID: "a", CreatedAt: t0}},
	}
	raw, err := json.Marshal(legacy)
	if err != nil {
		t.Fatal(err)
	}
	legacyPath := filepath.Join(dir, legacyFile)
	if err := os.WriteFile(legacyPath, raw, 0o600); err != nil {
		t.Fatal(err)
	}

	s, err := Open(filepath.Join(dir, "silicium.db"))
	if err != nil {
		t.Fatal(err)
	}
	if _, err := s.UserByEmail("old@example.com"); err != nil {
		t.Fatalf("legacy user not imported: %v", err)
	}
	if _, err := s.JobOf("a", "j"); err != nil {
		t.Fatalf("legacy job not imported: %v", err)
	}
	if _, err := os.Stat(legacyPath); !os.IsNotExist(err) {
		t.Fatalf("store.json still in place: %v", err)
	}
	if _, err := os.Stat(legacyPath + ".migrated"); err != nil {
		t.Fatalf("store.json.migrated missing: %v", err)
	}
	s.Close()

	// A second start with a fresh store.json must not import it again.
	if err := os.WriteFile(legacyPath, []byte(`{"users":{"b":{"id":"b","email":"new@example.com"}}}`), 0o600); err != nil {
		t.Fatal(err)
	}
	again, err := Open(filepath.Join(dir, "silicium.db"))
	if err != nil {
		t.Fatal(err)
	}
	defer again.Close()
	if _, err := again.UserByEmail("new@example.com"); err == nil {
		t.Fatal("second import happened")
	}
}

// A legacy file must not be mixed into a database that already holds data.
func TestLegacyJSONIsRefusedOnANonEmptyDatabase(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "silicium.db")
	s, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	if err := s.CreateUser(User{ID: "a", Email: "x@y.z", CreatedAt: t0}); err != nil {
		t.Fatal(err)
	}
	s.Close()
	if err := os.WriteFile(filepath.Join(dir, legacyFile), []byte(`{}`), 0o600); err != nil {
		t.Fatal(err)
	}
	if _, err := Open(path); err == nil {
		t.Fatal("expected a refusal to import into a non-empty database")
	}
}

func TestDatabaseFileIsPrivate(t *testing.T) {
	if os.PathSeparator == '\\' {
		t.Skip("permissions are not POSIX on Windows")
	}
	path := filepath.Join(t.TempDir(), "silicium.db")
	s, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	s.Close()
	info, err := os.Stat(path)
	if err != nil {
		t.Fatal(err)
	}
	if info.Mode().Perm() != 0o600 {
		t.Fatalf("mode %v, want 0600", info.Mode().Perm())
	}
}
