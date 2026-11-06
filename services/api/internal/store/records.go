package store

import (
	"sort"
	"time"
)

// CreateUser adds an account; the e-mail (already normalised) must be unused.
func (s *Store) CreateUser(u User) error {
	return s.update(func(d *snapshot) error {
		for _, other := range d.Users {
			// Empty fields mean "not set": wallet accounts have no e-mail, and vice versa.
			if u.Email != "" && other.Email == u.Email {
				return ErrExists
			}
			if u.Address != "" && other.Address == u.Address {
				return ErrExists
			}
		}
		d.Users[u.ID] = u
		return nil
	})
}

// UserByAddress finds an account linked to a wallet address (canonical base58).
func (s *Store) UserByAddress(address string) (u User, err error) {
	err = ErrNotFound
	if address == "" {
		return u, err
	}
	s.read(func(d *snapshot) {
		for _, candidate := range d.Users {
			if candidate.Address == address {
				u, err = candidate, nil
			}
		}
	})
	return
}

// UserByEmail finds an account by normalised e-mail.
func (s *Store) UserByEmail(email string) (u User, err error) {
	err = ErrNotFound
	if email == "" {
		return u, err // wallet accounts have no e-mail: never match them
	}
	s.read(func(d *snapshot) {
		for _, candidate := range d.Users {
			if candidate.Email == email {
				u, err = candidate, nil
			}
		}
	})
	return
}

// UserByID finds an account by id.
func (s *Store) UserByID(id string) (u User, err error) {
	err = ErrNotFound
	s.read(func(d *snapshot) {
		if found, ok := d.Users[id]; ok {
			u, err = found, nil
		}
	})
	return
}

// PutSession stores a session and drops expired ones.
func (s *Store) PutSession(session Session, now time.Time) error {
	return s.update(func(d *snapshot) error {
		for key, other := range d.Sessions {
			if !other.ExpiresAt.After(now) {
				delete(d.Sessions, key)
			}
		}
		d.Sessions[session.TokenHash] = session
		return nil
	})
}

// SessionByHash returns a session that has not expired.
func (s *Store) SessionByHash(hash string, now time.Time) (session Session, err error) {
	err = ErrNotFound
	s.read(func(d *snapshot) {
		if found, ok := d.Sessions[hash]; ok && found.ExpiresAt.After(now) {
			session, err = found, nil
		}
	})
	return
}

// DeleteSession ends a session (no error if it is already gone).
func (s *Store) DeleteSession(hash string) error {
	return s.update(func(d *snapshot) error {
		delete(d.Sessions, hash)
		return nil
	})
}

// PutLink stores a one-time link code and drops expired ones.
func (s *Store) PutLink(link Link, now time.Time) error {
	return s.update(func(d *snapshot) error {
		for key, other := range d.Links {
			if !other.ExpiresAt.After(now) {
				delete(d.Links, key)
			}
		}
		d.Links[link.CodeHash] = link
		return nil
	})
}

// ConsumeLink atomically uses a link code and registers the device it unlocks. The code
// cannot be used twice, and the key cannot be linked twice.
func (s *Store) ConsumeLink(codeHash string, device Device, now time.Time) (Device, error) {
	err := s.update(func(d *snapshot) error {
		link, ok := d.Links[codeHash]
		if !ok || !link.ExpiresAt.After(now) {
			return ErrExpired
		}
		for _, other := range d.Devices {
			if other.PublicKey == device.PublicKey && other.RevokedAt == nil {
				return ErrExists
			}
		}
		delete(d.Links, codeHash)
		device.UserID = link.UserID
		d.Devices[device.ID] = device
		return nil
	})
	if err != nil {
		return Device{}, err
	}
	return device, nil
}

// DevicesOf lists the devices of a user, oldest first.
func (s *Store) DevicesOf(userID string) (out []Device) {
	s.read(func(d *snapshot) {
		for _, device := range d.Devices {
			if device.UserID == userID {
				out = append(out, device)
			}
		}
	})
	sort.Slice(out, func(i, j int) bool { return out[i].LinkedAt.Before(out[j].LinkedAt) })
	return out
}

// RevokeDevice marks a device of this user as revoked.
func (s *Store) RevokeDevice(userID, id string, now time.Time) error {
	return s.update(func(d *snapshot) error {
		device, ok := d.Devices[id]
		if !ok || device.UserID != userID {
			return ErrNotFound
		}
		if device.RevokedAt == nil {
			device.RevokedAt = &now
			d.Devices[id] = device
		}
		return nil
	})
}

// AddJob records a job for a user. Recording the same job twice for the same user is a
// no-op; a job already recorded by another user is refused.
func (s *Store) AddJob(job Job) error {
	return s.update(func(d *snapshot) error {
		if existing, ok := d.Jobs[job.ID]; ok {
			if existing.UserID == job.UserID {
				return nil
			}
			return ErrExists
		}
		d.Jobs[job.ID] = job
		return nil
	})
}

// JobsOf lists the jobs of a user, newest first.
func (s *Store) JobsOf(userID string) (out []Job) {
	s.read(func(d *snapshot) {
		for _, job := range d.Jobs {
			if job.UserID == userID {
				out = append(out, job)
			}
		}
	})
	sort.Slice(out, func(i, j int) bool { return out[i].CreatedAt.After(out[j].CreatedAt) })
	return out
}

// JobOf returns a job only if it belongs to the user.
func (s *Store) JobOf(userID, id string) (job Job, err error) {
	err = ErrNotFound
	s.read(func(d *snapshot) {
		if found, ok := d.Jobs[id]; ok && found.UserID == userID {
			job, err = found, nil
		}
	})
	return
}

// Export is everything the service holds about one account, minus secrets (no password
// hash, no session tokens, no link code hashes).
type Export struct {
	ID        string    `json:"id"`
	Email     string    `json:"email,omitempty"`
	Address   string    `json:"address,omitempty"`
	CreatedAt time.Time `json:"created_at"`
	Devices   []Device  `json:"devices"`
	Jobs      []Job     `json:"jobs"`
}

// ExportOf gathers the personal data of a user (GDPR art. 15 and 20).
func (s *Store) ExportOf(userID string) (Export, error) {
	user, err := s.UserByID(userID)
	if err != nil {
		return Export{}, err
	}
	out := Export{ID: user.ID, Email: user.Email, Address: user.Address, CreatedAt: user.CreatedAt}
	out.Devices = s.DevicesOf(userID)
	out.Jobs = s.JobsOf(userID)
	if out.Devices == nil {
		out.Devices = []Device{}
	}
	if out.Jobs == nil {
		out.Jobs = []Job{}
	}
	return out, nil
}

// DeleteUser erases the account and everything linked to it: sessions, link codes,
// devices and the job index (GDPR art. 17). The ledger on the network is not ours to erase.
func (s *Store) DeleteUser(userID string) error {
	return s.update(func(d *snapshot) error {
		if _, ok := d.Users[userID]; !ok {
			return ErrNotFound
		}
		delete(d.Users, userID)
		for key, session := range d.Sessions {
			if session.UserID == userID {
				delete(d.Sessions, key)
			}
		}
		for key, link := range d.Links {
			if link.UserID == userID {
				delete(d.Links, key)
			}
		}
		for key, device := range d.Devices {
			if device.UserID == userID {
				delete(d.Devices, key)
			}
		}
		for key, job := range d.Jobs {
			if job.UserID == userID {
				delete(d.Jobs, key)
			}
		}
		return nil
	})
}
