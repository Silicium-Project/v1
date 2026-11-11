package store

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

const (
	// legacyFile is the former JSON store, looked for next to the database.
	legacyFile = "store.json"
	// legacyMeta records that the import happened. Once it is set, store.json is never read again.
	legacyMeta = "legacy_import"
)

func dirOf(path string) string { return filepath.Dir(path) }

// importLegacy copies the former store.json, if present, into the database in one transaction,
// then renames the file to store.json.migrated (kept, never deleted). It refuses to import into a
// database that already holds data: the two would be silently mixed.
func (s *Store) importLegacy(dir string) error {
	if _, done, err := metaValue(s.db, legacyMeta); err != nil || done {
		return err
	}
	path := filepath.Join(dir, legacyFile)
	if _, err := os.Stat(path); errors.Is(err, os.ErrNotExist) {
		return nil
	} else if err != nil {
		return err
	}
	if !s.data.empty() {
		return fmt.Errorf("%s et la base contiennent déjà des données : import refusé", path)
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	var legacy snapshot
	if err := json.Unmarshal(raw, &legacy); err != nil {
		return fmt.Errorf("%s illisible : %w", path, err)
	}
	legacy.fillNil()
	after, err := encode(&legacy)
	if err != nil {
		return err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	if err := applyRows(tx, encodings{}, after, &legacy); err != nil {
		return err
	}
	if err := setMeta(tx, legacyMeta, time.Now().UTC().Format(timeLayout)); err != nil {
		return err
	}
	if err := tx.Commit(); err != nil {
		return err
	}
	s.data = legacy
	return os.Rename(path, path+".migrated")
}
