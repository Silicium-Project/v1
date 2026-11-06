// Package store keeps accounts, sessions, device links and job bookkeeping.
//
// It is a single JSON file written atomically (temp file, fsync, rename) after every
// change. That is enough for one API instance and a few thousand accounts; a SQL backend
// can replace it behind the same methods (see docs/technical/api-service.md).
package store

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"sync"
)

var (
	// ErrNotFound means the record does not exist (or is not yours).
	ErrNotFound = errors.New("not found")
	// ErrExists means a unique field is already taken.
	ErrExists = errors.New("already exists")
	// ErrExpired means a one-time record is past its deadline or already used.
	ErrExpired = errors.New("expired or used")
)

type snapshot struct {
	Users    map[string]User    `json:"users"`   // by id
	Sessions map[string]Session `json:"session"` // by token hash
	Links    map[string]Link    `json:"links"`   // by code hash
	Devices  map[string]Device  `json:"devices"` // by id
	Jobs     map[string]Job     `json:"jobs"`    // by job id
}

// Store is safe for concurrent use.
type Store struct {
	mu   sync.Mutex
	path string // empty: memory only
	data snapshot
}

// Open loads the file at path (created on first change). An empty path gives an
// in-memory store, for tests.
func Open(path string) (*Store, error) {
	s := &Store{path: path, data: emptySnapshot()}
	if path == "" {
		return s, nil
	}
	raw, err := os.ReadFile(path)
	switch {
	case errors.Is(err, os.ErrNotExist):
		return s, nil
	case err != nil:
		return nil, err
	}
	if err := json.Unmarshal(raw, &s.data); err != nil {
		return nil, err
	}
	s.data.fillNil()
	return s, nil
}

func emptySnapshot() snapshot {
	var s snapshot
	s.fillNil()
	return s
}

func (s *snapshot) fillNil() {
	if s.Users == nil {
		s.Users = map[string]User{}
	}
	if s.Sessions == nil {
		s.Sessions = map[string]Session{}
	}
	if s.Links == nil {
		s.Links = map[string]Link{}
	}
	if s.Devices == nil {
		s.Devices = map[string]Device{}
	}
	if s.Jobs == nil {
		s.Jobs = map[string]Job{}
	}
}

// update runs change under the lock and saves if it succeeds. If saving fails the change
// is rolled back in memory, so memory never gets ahead of the disk.
func (s *Store) update(change func(*snapshot) error) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	backup, err := s.clone()
	if err != nil {
		return err
	}
	if err := change(&s.data); err != nil {
		s.data = backup
		return err
	}
	if err := s.save(); err != nil {
		s.data = backup
		return err
	}
	return nil
}

func (s *Store) read(look func(*snapshot)) {
	s.mu.Lock()
	defer s.mu.Unlock()
	look(&s.data)
}

func (s *Store) clone() (snapshot, error) {
	raw, err := json.Marshal(s.data)
	if err != nil {
		return snapshot{}, err
	}
	var copy snapshot
	if err := json.Unmarshal(raw, &copy); err != nil {
		return snapshot{}, err
	}
	copy.fillNil()
	return copy, nil
}

func (s *Store) save() error {
	if s.path == "" {
		return nil
	}
	raw, err := json.Marshal(s.data)
	if err != nil {
		return err
	}
	dir := filepath.Dir(s.path)
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return err
	}
	tmp, err := os.CreateTemp(dir, ".store-*")
	if err != nil {
		return err
	}
	defer os.Remove(tmp.Name())
	if _, err := tmp.Write(raw); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Sync(); err != nil {
		tmp.Close()
		return err
	}
	if err := tmp.Close(); err != nil {
		return err
	}
	return os.Rename(tmp.Name(), s.path)
}
