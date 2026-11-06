package auth

import (
	"errors"
	"time"

	"sil/api/internal/store"
	"sil/api/internal/wallet"
)

// WalletChallenge issues a single-use message for a wallet to sign. The purpose decides what
// the signature authorises (login or account deletion).
func (s *Service) WalletChallenge(address string, purpose wallet.Purpose) (nonce, message string, expires time.Time, err error) {
	raw, err := wallet.DecodeBase58(address)
	if err != nil || len(raw) != 32 {
		return "", "", time.Time{}, ErrInvalidInput
	}
	var key [32]byte
	copy(key[:], raw)
	nonce, message, expires, err = s.challenges.Issue(key, purpose)
	return nonce, message, expires, err
}

// LoginWallet opens a session for the owner of address. The first valid login creates the
// account; there is no e-mail and no password for a wallet account.
func (s *Service) LoginWallet(address, nonce string, signature []byte) (token string, expires time.Time, err error) {
	key, canonical, err := parseAddress(address)
	if err != nil {
		return "", time.Time{}, ErrInvalidCredentials
	}
	message, err := s.challenges.Consume(nonce, key, wallet.PurposeLogin)
	if err != nil || !wallet.Verify(key, message, signature) {
		return "", time.Time{}, ErrInvalidCredentials
	}
	user, err := s.store.UserByAddress(canonical)
	if errors.Is(err, store.ErrNotFound) {
		user = store.User{ID: randomID(), Address: canonical, CreatedAt: s.now().UTC()}
		if err := s.store.CreateUser(user); err != nil {
			return "", time.Time{}, err
		}
	} else if err != nil {
		return "", time.Time{}, err
	}
	return s.openSession(user.ID)
}

// DeleteChallenge issues a challenge for deleting a wallet account (password-less accounts).
func (s *Service) DeleteChallenge(userID string) (nonce, message string, expires time.Time, err error) {
	user, err := s.store.UserByID(userID)
	if err != nil {
		return "", "", time.Time{}, err
	}
	if user.Address == "" {
		return "", "", time.Time{}, ErrInvalidInput
	}
	return s.WalletChallenge(user.Address, wallet.PurposeDelete)
}

// DeleteAccountWithWallet erases a wallet account after a fresh delete signature.
func (s *Service) DeleteAccountWithWallet(userID, nonce string, signature []byte) error {
	user, err := s.store.UserByID(userID)
	if err != nil {
		return err
	}
	key, _, err := parseAddress(user.Address)
	if err != nil {
		return ErrInvalidCredentials
	}
	message, err := s.challenges.Consume(nonce, key, wallet.PurposeDelete)
	if err != nil || !wallet.Verify(key, message, signature) {
		return ErrInvalidCredentials
	}
	return s.store.DeleteUser(userID)
}

// parseAddress decodes a base58 address and returns its key and its canonical text.
func parseAddress(address string) ([32]byte, string, error) {
	var key [32]byte
	raw, err := wallet.DecodeBase58(address)
	if err != nil || len(raw) != 32 {
		return key, "", ErrInvalidInput
	}
	copy(key[:], raw)
	return key, wallet.EncodeBase58(raw), nil
}
