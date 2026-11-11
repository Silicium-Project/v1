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
	// Telemetry is the last report the machine sent (see devices.Service.Report). Nil until one arrives.
	Telemetry *Telemetry `json:"telemetry,omitempty"`
}

// Telemetry is what a machine reports about itself. ObservedAt is the machine clock; ReceivedAt is ours.
type Telemetry struct {
	ObservedAt      time.Time `json:"observed_at"`
	ReceivedAt      time.Time `json:"received_at"`
	Version         string    `json:"version"`
	CPUPct          float64   `json:"cpu_pct"`
	MemPct          float64   `json:"mem_pct"`
	CPUCores        int       `json:"cpu_cores"`
	Compute         bool      `json:"compute"`
	Verify          bool      `json:"verify"`
	TasksActive     int       `json:"tasks_active"`
	TasksInProgress int       `json:"tasks_in_progress"`
	TasksFinalized  uint64    `json:"tasks_finalized"`
	TasksOther      uint64    `json:"tasks_other"`
	// Reputation as the ledger shows it: score and the good and bad attempts behind it.
	ReputationScore *float64 `json:"reputation_score,omitempty"`
	ReputationGood  *float64 `json:"reputation_good,omitempty"`
	ReputationBad   *float64 `json:"reputation_bad,omitempty"`
	// Earnings confirmed on chain only: nothing counts before its fragment is settled.
	WorkerLamports   *uint64 `json:"worker_lamports,omitempty"`
	VerifierLamports *uint64 `json:"verifier_lamports,omitempty"`
	FragmentsSettled *uint64 `json:"fragments_settled,omitempty"`
}

// Job is local bookkeeping of the jobs a user submitted. It is an index for "my jobs",
// not an authority: ownership of a job is the requester signature in the ledger.
type Job struct {
	ID        string    `json:"id"` // 64 lower-case hex characters
	UserID    string    `json:"user_id"`
	Label     string    `json:"label"`
	CreatedAt time.Time `json:"created_at"`
	// Origins gives, for each passage of an embedding job, where it comes from (file, page, line).
	// Owner only; never part of the job list.
	Origins []string `json:"origins,omitempty"`
}
