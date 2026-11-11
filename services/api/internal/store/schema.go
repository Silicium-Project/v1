package store

import (
	"database/sql"
	"errors"
	"os"
	"path/filepath"

	// Driver SQLite en Go pur (pas de cgo : le binaire reste statique, comme le reste de l'API).
	_ "modernc.org/sqlite"
)

// schemaSQL is run at every start. CREATE ... IF NOT EXISTS keeps it idempotent.
// Times are stored as RFC 3339 text in UTC. Empty optional fields are NULL, so that UNIQUE
// constraints ignore them (wallet accounts have no e-mail, and the reverse).
var schemaSQL = []string{
	`CREATE TABLE IF NOT EXISTS users (
		id         TEXT PRIMARY KEY,
		email      TEXT UNIQUE,
		address    TEXT UNIQUE,
		salt       BLOB,
		hash       BLOB,
		iter       INTEGER NOT NULL DEFAULT 0,
		created_at TEXT NOT NULL
	)`,
	`CREATE TABLE IF NOT EXISTS sessions (
		token_hash TEXT PRIMARY KEY,
		user_id    TEXT NOT NULL,
		expires_at TEXT NOT NULL
	)`,
	`CREATE INDEX IF NOT EXISTS sessions_user ON sessions (user_id)`,
	`CREATE TABLE IF NOT EXISTS links (
		code_hash  TEXT PRIMARY KEY,
		user_id    TEXT NOT NULL,
		expires_at TEXT NOT NULL
	)`,
	`CREATE TABLE IF NOT EXISTS devices (
		id         TEXT PRIMARY KEY,
		user_id    TEXT NOT NULL,
		public_key TEXT NOT NULL,
		label      TEXT NOT NULL DEFAULT '',
		linked_at  TEXT NOT NULL,
		revoked_at TEXT,
		telemetry  TEXT
	)`,
	`CREATE INDEX IF NOT EXISTS devices_user ON devices (user_id)`,
	`CREATE TABLE IF NOT EXISTS jobs (
		id         TEXT PRIMARY KEY,
		user_id    TEXT NOT NULL,
		label      TEXT NOT NULL DEFAULT '',
		created_at TEXT NOT NULL
	)`,
	`CREATE INDEX IF NOT EXISTS jobs_user ON jobs (user_id)`,
	`CREATE TABLE IF NOT EXISTS meta (
		key   TEXT PRIMARY KEY,
		value TEXT NOT NULL
	)`,
}

// openDB opens (and creates, mode 0600) the SQLite database at path. The file holds password
// hashes and session hashes: it must not be readable by other users.
func openDB(path string) (*sql.DB, error) {
	if err := os.MkdirAll(filepath.Dir(path), 0o700); err != nil {
		return nil, err
	}
	if _, err := os.Stat(path); errors.Is(err, os.ErrNotExist) {
		f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY, 0o600)
		if err != nil {
			return nil, err
		}
		if err := f.Close(); err != nil {
			return nil, err
		}
	}
	dsn := "file:" + filepath.ToSlash(path) +
		"?_pragma=busy_timeout(5000)&_pragma=journal_mode(WAL)&_pragma=synchronous(FULL)&_pragma=foreign_keys(1)"
	db, err := sql.Open("sqlite", dsn)
	if err != nil {
		return nil, err
	}
	// One connection: writes are already serialised by Store.mu, and a single connection
	// makes a transaction and a read on the same goroutine impossible to deadlock.
	db.SetMaxOpenConns(1)
	for _, statement := range schemaSQL {
		if _, err := db.Exec(statement); err != nil {
			db.Close()
			return nil, err
		}
	}
	if err := addColumnIfMissing(db, "jobs", "origins", "TEXT NOT NULL DEFAULT '[]'"); err != nil {
		db.Close()
		return nil, err
	}
	return db, nil
}

// addColumnIfMissing adds a column to an existing table (SQLite has no ADD COLUMN IF NOT EXISTS).
func addColumnIfMissing(db *sql.DB, table, column, definition string) error {
	rows, err := db.Query("SELECT name FROM pragma_table_info(?)", table)
	if err != nil {
		return err
	}
	defer rows.Close()
	for rows.Next() {
		var name string
		if err := rows.Scan(&name); err != nil {
			return err
		}
		if name == column {
			return nil
		}
	}
	if err := rows.Err(); err != nil {
		return err
	}
	_, err = db.Exec("ALTER TABLE " + table + " ADD COLUMN " + column + " " + definition)
	return err
}

// querier is implemented by *sql.DB and *sql.Tx.
type querier interface {
	Exec(query string, args ...any) (sql.Result, error)
	QueryRow(query string, args ...any) *sql.Row
}

// metaValue returns the value stored under key, and whether it exists.
func metaValue(q querier, key string) (string, bool, error) {
	var value string
	err := q.QueryRow(`SELECT value FROM meta WHERE key = ?`, key).Scan(&value)
	if errors.Is(err, sql.ErrNoRows) {
		return "", false, nil
	}
	if err != nil {
		return "", false, err
	}
	return value, true, nil
}

func setMeta(q querier, key, value string) error {
	_, err := q.Exec(`INSERT INTO meta (key, value) VALUES (?, ?)
		ON CONFLICT(key) DO UPDATE SET value = excluded.value`, key, value)
	return err
}
