package store

import "time"

// User is an account. Only a salted, slow hash of the password is kept.
type User struct {
	ID        string    `json:"id"`
	Email     string    `json:"email,omitempty"` // lower case, unique when set
	Address   string    `json:"address,omitempty"`
	Salt      []byte    `json:"salt,omitempty"`
	Hash      []byte    `json:"hash,omitempty"`
	Iter      int       `json:"iter,omitempty"`
	CreatedAt time.Time `json:"created_at"`
}

// HasPassword tells whether the account can sign in with an e-mail and a password.
func (u User) HasPassword() bool { return u.Email != "" && len(u.Hash) > 0 }

// Session is a bearer session. Only the SHA-256 of the token is stored, so a leaked
// store file cannot be used to log in.
type Session struct {
	TokenHash string    `json:"token_hash"`
	UserID    string    `json:"user_id"`
	ExpiresAt time.Time `json:"expires_at"`
}

// Link is a one-time code that lets a machine prove it belongs to an account.
type Link struct {
	CodeHash  string    `json:"code_hash"`
	UserID    string    `json:"user_id"`
	ExpiresAt time.Time `json:"expires_at"`
}

// Device is a machine (an Ed25519 key) linked to an account.
type Device struct {
	ID        string     `json:"id"`
	UserID    string     `json:"user_id"`
	PublicKey string     `json:"public_key"` // 64 lower-case hex characters
	Label     string     `json:"label"`
	LinkedAt  time.Time  `json:"linked_at"`
	RevokedAt *time.Time `json:"revoked_at,omitempty"`
}

// Job is local bookkeeping of the jobs a user submitted. It is an index for "my jobs",
// not an authority: ownership of a job is the requester signature in the ledger.
type Job struct {
	ID        string    `json:"id"` // 64 lower-case hex characters
	UserID    string    `json:"user_id"`
	Label     string    `json:"label"`
	CreatedAt time.Time `json:"created_at"`
}
