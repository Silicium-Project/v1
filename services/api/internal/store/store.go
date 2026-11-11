// Package store keeps accounts, sessions, device links and job bookkeeping.
//
// The data lives in one SQLite database (pure Go driver, no cgo). The service works on an
// in-memory copy; each change is written back in one transaction that touches only the rows
// that changed. If the write fails, the memory copy is rolled back, so memory never gets
// ahead of the disk. A store.json found next to the database (the former format) is
// imported once, then renamed store.json.migrated.
package store

import (
	"database/sql"
	"encoding/json"
	"errors"
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

// snapshot is the in-memory copy of the database. The JSON tags are kept for the legacy
// store.json import.
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
	db   *sql.DB // nil: memory only (tests)
	data snapshot
}

// Open opens (creating it if needed) the database at path. An empty path gives an in-memory
// store, for tests.
func Open(path string) (*Store, error) {
	s := &Store{data: emptySnapshot()}
	if path == "" {
		return s, nil
	}
	db, err := openDB(path)
	if err != nil {
		return nil, err
	}
	s.db = db
	if err := loadAll(db, &s.data); err != nil {
		db.Close()
		return nil, err
	}
	if err := s.importLegacy(dirOf(path)); err != nil {
		db.Close()
		return nil, err
	}
	return s, nil
}

// Close releases the database. The store must not be used afterwards.
func (s *Store) Close() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.db == nil {
		return nil
	}
	return s.db.Close()
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

// update runs change under the lock and persists the difference. If the change or the write
// fails, memory goes back to its state before the change.
func (s *Store) update(change func(*snapshot) error) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	backup, err := s.clone()
	if err != nil {
		return err
	}
	var before encodings
	if s.db != nil {
		if before, err = encode(&s.data); err != nil {
			return err
		}
	}
	if err := change(&s.data); err != nil {
		s.data = backup
		return err
	}
	if s.db != nil {
		if err := s.persist(before); err != nil {
			s.data = backup
			return err
		}
	}
	return nil
}

// persist writes, in one transaction, the rows that differ from before.
func (s *Store) persist(before encodings) error {
	after, err := encode(&s.data)
	if err != nil {
		return err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback() // no-op once committed
	if err := applyRows(tx, before, after, &s.data); err != nil {
		return err
	}
	return tx.Commit()
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

func (d *snapshot) empty() bool {
	return len(d.Users)+len(d.Sessions)+len(d.Links)+len(d.Devices)+len(d.Jobs) == 0
}
