package store

import (
	"database/sql"
	"encoding/json"
	"time"
)

const timeLayout = time.RFC3339Nano

func nullable(s string) any {
	if s == "" {
		return nil
	}
	return s
}

func formatTime(t time.Time) string { return t.UTC().Format(timeLayout) }

func parseTime(s string) (time.Time, error) { return time.Parse(timeLayout, s) }

func writeUser(tx *sql.Tx, u User) error {
	_, err := tx.Exec(`INSERT INTO users (id, email, address, salt, hash, iter, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET email = excluded.email, address = excluded.address,
			salt = excluded.salt, hash = excluded.hash, iter = excluded.iter,
			created_at = excluded.created_at`,
		u.ID, nullable(u.Email), nullable(u.Address), u.Salt, u.Hash, u.Iter, formatTime(u.CreatedAt))
	return err
}

func writeSession(tx *sql.Tx, s Session) error {
	_, err := tx.Exec(`INSERT INTO sessions (token_hash, user_id, expires_at) VALUES (?, ?, ?)
		ON CONFLICT(token_hash) DO UPDATE SET user_id = excluded.user_id, expires_at = excluded.expires_at`,
		s.TokenHash, s.UserID, formatTime(s.ExpiresAt))
	return err
}

func writeLink(tx *sql.Tx, l Link) error {
	_, err := tx.Exec(`INSERT INTO links (code_hash, user_id, expires_at) VALUES (?, ?, ?)
		ON CONFLICT(code_hash) DO UPDATE SET user_id = excluded.user_id, expires_at = excluded.expires_at`,
		l.CodeHash, l.UserID, formatTime(l.ExpiresAt))
	return err
}

func writeDevice(tx *sql.Tx, d Device) error {
	var revoked any
	if d.RevokedAt != nil {
		revoked = formatTime(*d.RevokedAt)
	}
	var telemetry any
	if d.Telemetry != nil {
		raw, err := json.Marshal(d.Telemetry)
		if err != nil {
			return err
		}
		telemetry = string(raw)
	}
	_, err := tx.Exec(`INSERT INTO devices (id, user_id, public_key, label, linked_at, revoked_at, telemetry)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET user_id = excluded.user_id, public_key = excluded.public_key,
			label = excluded.label, linked_at = excluded.linked_at, revoked_at = excluded.revoked_at,
			telemetry = excluded.telemetry`,
		d.ID, d.UserID, d.PublicKey, d.Label, formatTime(d.LinkedAt), revoked, telemetry)
	return err
}

func writeJob(tx *sql.Tx, j Job) error {
	origins := j.Origins
	if origins == nil {
		origins = []string{}
	}
	raw, err := json.Marshal(origins)
	if err != nil {
		return err
	}
	_, err = tx.Exec(`INSERT INTO jobs (id, user_id, label, created_at, origins) VALUES (?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET user_id = excluded.user_id, label = excluded.label,
			created_at = excluded.created_at, origins = excluded.origins`,
		j.ID, j.UserID, j.Label, formatTime(j.CreatedAt), string(raw))
	return err
}

// loadAll reads every table into d. It runs once, at Open.
func loadAll(db *sql.DB, d *snapshot) error {
	if err := loadRows(db, `SELECT id, COALESCE(email, ''), COALESCE(address, ''), salt, hash, iter, created_at FROM users`,
		func(scan func(...any) error) error {
			var u User
			var created string
			if err := scan(&u.ID, &u.Email, &u.Address, &u.Salt, &u.Hash, &u.Iter, &created); err != nil {
				return err
			}
			t, err := parseTime(created)
			if err != nil {
				return err
			}
			u.CreatedAt = t
			d.Users[u.ID] = u
			return nil
		}); err != nil {
		return err
	}
	if err := loadRows(db, `SELECT token_hash, user_id, expires_at FROM sessions`,
		func(scan func(...any) error) error {
			var s Session
			var expires string
			if err := scan(&s.TokenHash, &s.UserID, &expires); err != nil {
				return err
			}
			t, err := parseTime(expires)
			if err != nil {
				return err
			}
			s.ExpiresAt = t
			d.Sessions[s.TokenHash] = s
			return nil
		}); err != nil {
		return err
	}
	if err := loadRows(db, `SELECT code_hash, user_id, expires_at FROM links`,
		func(scan func(...any) error) error {
			var l Link
			var expires string
			if err := scan(&l.CodeHash, &l.UserID, &expires); err != nil {
				return err
			}
			t, err := parseTime(expires)
			if err != nil {
				return err
			}
			l.ExpiresAt = t
			d.Links[l.CodeHash] = l
			return nil
		}); err != nil {
		return err
	}
	if err := loadRows(db, `SELECT id, user_id, public_key, label, linked_at, revoked_at, telemetry FROM devices`,
		func(scan func(...any) error) error {
			var dev Device
			var linked string
			var revoked, telemetry sql.NullString
			if err := scan(&dev.ID, &dev.UserID, &dev.PublicKey, &dev.Label, &linked, &revoked, &telemetry); err != nil {
				return err
			}
			t, err := parseTime(linked)
			if err != nil {
				return err
			}
			dev.LinkedAt = t
			if revoked.Valid {
				r, err := parseTime(revoked.String)
				if err != nil {
					return err
				}
				dev.RevokedAt = &r
			}
			if telemetry.Valid {
				dev.Telemetry = &Telemetry{}
				if err := json.Unmarshal([]byte(telemetry.String), dev.Telemetry); err != nil {
					return err
				}
			}
			d.Devices[dev.ID] = dev
			return nil
		}); err != nil {
		return err
	}
	return loadRows(db, `SELECT id, user_id, label, created_at, origins FROM jobs`,
		func(scan func(...any) error) error {
			var j Job
			var created, origins string
			if err := scan(&j.ID, &j.UserID, &j.Label, &created, &origins); err != nil {
				return err
			}
			t, err := parseTime(created)
			if err != nil {
				return err
			}
			j.CreatedAt = t
			if err := json.Unmarshal([]byte(origins), &j.Origins); err != nil {
				return err
			}
			d.Jobs[j.ID] = j
			return nil
		})
}

// loadRows runs query and calls each with the scanner of one row.
func loadRows(db *sql.DB, query string, each func(scan func(...any) error) error) error {
	rows, err := db.Query(query)
	if err != nil {
		return err
	}
	defer rows.Close()
	for rows.Next() {
		if err := each(rows.Scan); err != nil {
			return err
		}
	}
	return rows.Err()
}
