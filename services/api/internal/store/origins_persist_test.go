package store

import (
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

// The origins of a job survive a restart, and a database created before the column existed
// gets it added.
func TestOriginsSurviveARestart(t *testing.T) {
	path := filepath.Join(t.TempDir(), "silicium.db")
	s, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	job := Job{ID: "ab" + repeat("cd", 31), UserID: "u1", Label: "notes", CreatedAt: time.Unix(1700000000, 0).UTC()}
	if err := s.AddJob(job); err != nil {
		t.Fatal(err)
	}
	if err := s.SetOrigins("u1", job.ID, []string{"a.pdf · page 1", "b.txt · ligne 2"}); err != nil {
		t.Fatal(err)
	}
	if err := s.Close(); err != nil {
		t.Fatal(err)
	}

	reopened, err := Open(path)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.Close()
	got, err := reopened.OriginsOf("u1", job.ID)
	if err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(got, []string{"a.pdf · page 1", "b.txt · ligne 2"}) {
		t.Fatalf("origins after restart: %q", got)
	}
	if _, err := reopened.OriginsOf("someone-else", job.ID); err != ErrNotFound {
		t.Fatalf("another user got the origins: %v", err)
	}
}

func repeat(s string, n int) string {
	out := ""
	for i := 0; i < n; i++ {
		out += s
	}
	return out
}
