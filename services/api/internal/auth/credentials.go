package auth

import (
	"crypto/subtle"
	"errors"

	"sil/api/internal/store"
)

// ErrWrongPassword: the current password given to change the credentials is wrong. It is not
// ErrInvalidCredentials, which ends the session on the client side.
var ErrWrongPassword = errors.New("wrong current password")

// SetCredentials attaches an e-mail and a password to an account, or changes them.
//
// An account that already has a password must confirm with it. A wallet account has none yet:
// it must set one. An empty newPassword keeps the current password, which is only possible once
// the account has one.
func (s *Service) SetCredentials(userID, email, currentPassword, newPassword string) error {
	email, err := NormalizeEmail(email)
	if err != nil {
		return ErrInvalidInput
	}
	user, err := s.store.UserByID(userID)
	if err != nil {
		return err
	}
	if user.HasPassword() {
		candidate, hashErr := hashPassword(currentPassword[:min(len(currentPassword), 4*maxPassword)], user.Salt, user.Iter)
		if hashErr != nil || subtle.ConstantTimeCompare(candidate, user.Hash) != 1 {
			return ErrWrongPassword
		}
	}
	salt, hash, iter := user.Salt, user.Hash, user.Iter
	if newPassword != "" || !user.HasPassword() {
		if !validPassword(newPassword) {
			return ErrInvalidInput
		}
		salt = randomBytes(16)
		hash, err = hashPassword(newPassword, salt, s.cfg.Iterations)
		if err != nil {
			return err
		}
		iter = s.cfg.Iterations
	}
	if err := s.store.SetCredentials(userID, email, salt, hash, iter); err != nil {
		if errors.Is(err, store.ErrExists) {
			return ErrEmailTaken
		}
		return err
	}
	return nil
}
