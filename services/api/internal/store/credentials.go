package store

// SetCredentials sets the e-mail and the password fields of an account. The e-mail must not
// belong to another account.
func (s *Store) SetCredentials(userID, email string, salt, hash []byte, iter int) error {
	return s.update(func(d *snapshot) error {
		user, ok := d.Users[userID]
		if !ok {
			return ErrNotFound
		}
		for id, other := range d.Users {
			if id != userID && other.Email == email {
				return ErrExists
			}
		}
		user.Email, user.Salt, user.Hash, user.Iter = email, salt, hash, iter
		d.Users[userID] = user
		return nil
	})
}
