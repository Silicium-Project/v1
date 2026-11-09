package store

import "time"

// LinkByCode returns a link code that is still usable (not used, not expired), without
// consuming it. ConsumeLink is the only way to use one up.
func (s *Store) LinkByCode(codeHash string, now time.Time) (link Link, err error) {
	err = ErrExpired
	s.read(func(d *snapshot) {
		if found, ok := d.Links[codeHash]; ok && found.ExpiresAt.After(now) {
			link, err = found, nil
		}
	})
	return link, err
}
