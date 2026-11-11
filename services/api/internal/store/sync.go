package store

import (
	"bytes"
	"database/sql"
	"encoding/json"
)

// encodings holds the JSON of every row, by table and key. Comparing two encodings tells
// whether a row changed, without a round trip through the database.
type encodings struct {
	users, sessions, links, devices, jobs map[string][]byte
}

func encode(d *snapshot) (encodings, error) {
	var (
		e   encodings
		err error
	)
	if e.users, err = encodeRows(d.Users); err != nil {
		return e, err
	}
	if e.sessions, err = encodeRows(d.Sessions); err != nil {
		return e, err
	}
	if e.links, err = encodeRows(d.Links); err != nil {
		return e, err
	}
	if e.devices, err = encodeRows(d.Devices); err != nil {
		return e, err
	}
	e.jobs, err = encodeRows(d.Jobs)
	return e, err
}

func encodeRows[T any](rows map[string]T) (map[string][]byte, error) {
	out := make(map[string][]byte, len(rows))
	for key, row := range rows {
		raw, err := json.Marshal(row)
		if err != nil {
			return nil, err
		}
		out[key] = raw
	}
	return out, nil
}

// table describes how one collection is stored: its table, its primary key, and how to
// write one row.
type table[T any] struct {
	name  string
	key   string
	write func(*sql.Tx, T) error
}

// syncRows writes, inside tx, the difference between before and after: rows that vanished
// are deleted, rows that are new or changed are upserted. Unchanged rows are not touched.
func syncRows[T any](tx *sql.Tx, t table[T], before, after map[string][]byte, rows map[string]T) error {
	for key := range before {
		if _, kept := after[key]; kept {
			continue
		}
		if _, err := tx.Exec("DELETE FROM "+t.name+" WHERE "+t.key+" = ?", key); err != nil {
			return err
		}
	}
	for key, raw := range after {
		if old, existed := before[key]; existed && bytes.Equal(old, raw) {
			continue
		}
		if err := t.write(tx, rows[key]); err != nil {
			return err
		}
	}
	return nil
}

// applyRows writes the difference between before and after for every table. Passing empty
// encodings as before writes the whole snapshot (used for the first import).
func applyRows(tx *sql.Tx, before, after encodings, d *snapshot) error {
	if err := syncRows(tx, table[Session]{"sessions", "token_hash", writeSession}, before.sessions, after.sessions, d.Sessions); err != nil {
		return err
	}
	if err := syncRows(tx, table[Link]{"links", "code_hash", writeLink}, before.links, after.links, d.Links); err != nil {
		return err
	}
	if err := syncRows(tx, table[Device]{"devices", "id", writeDevice}, before.devices, after.devices, d.Devices); err != nil {
		return err
	}
	if err := syncRows(tx, table[Job]{"jobs", "id", writeJob}, before.jobs, after.jobs, d.Jobs); err != nil {
		return err
	}
	// Users last. There are no foreign keys: the order only matters for UNIQUE e-mail and
	// address values that move from one account to another within the same change.
	return syncRows(tx, table[User]{"users", "id", writeUser}, before.users, after.users, d.Users)
}
