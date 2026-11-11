package devices

import (
	"crypto/ed25519"
	"encoding/hex"
	"encoding/json"
	"errors"
	"math"
	"strings"
	"time"

	"sil/api/internal/store"
)

// TelemetryContext prefixes the signed telemetry message. It matches the Rust side
// (sil-node telemetry.rs: "sil/v1/telemetry\n" followed by the payload).
const TelemetryContext = "sil/v1/telemetry\n"

// TelemetrySkew is how far the machine clock may be from ours.
const TelemetrySkew = 5 * time.Minute

const maxVersionRunes = 32

var (
	ErrUnknownDevice = errors.New("unknown device")
	ErrStaleReport   = errors.New("stale telemetry")
)

// telemetryPayload is what a machine reports about itself. Unknown fields are refused.
type telemetryPayload struct {
	ObservedAtMs    int64      `json:"observed_at_ms"`
	Version         string     `json:"version"`
	CPUPct          float64    `json:"cpu_pct"`
	MemPct          float64    `json:"mem_pct"`
	CPUCores        int        `json:"cpu_cores"`
	Roles           roles      `json:"roles"`
	TasksActive     int        `json:"tasks_active"`
	TasksInProgress int        `json:"tasks_in_progress"`
	TasksFinalized  uint64     `json:"tasks_finalized"`
	TasksOther      uint64     `json:"tasks_other"`
	Reputation      reputation `json:"reputation"`
	Earnings        earnings   `json:"earnings"`
}

// reputation and earnings are optional: a node that predates them does not send them.
type reputation struct {
	Score *float64 `json:"score"`
	Good  *float64 `json:"good"`
	Bad   *float64 `json:"bad"`
}

type earnings struct {
	WorkerLamports   *uint64 `json:"worker_lamports"`
	VerifierLamports *uint64 `json:"verifier_lamports"`
	FragmentsSettled *uint64 `json:"fragments_settled"`
}

type roles struct {
	Compute bool `json:"compute"`
	Verify  bool `json:"verify"`
}

// ReportTelemetry checks the signature of a machine's report and keeps it for the device that
// holds the key. The signature covers TelemetryContext followed by the payload bytes, exactly as
// sent. A revoked or unknown key is refused like a bad signature would be.
func (s *Service) ReportTelemetry(publicKeyHex, payload, signatureHex string) error {
	if publicKeyHex != strings.ToLower(publicKeyHex) {
		return ErrInvalidInput
	}
	key, err := hex.DecodeString(publicKeyHex)
	if err != nil || len(key) != ed25519.PublicKeySize {
		return ErrInvalidInput
	}
	signature, err := hex.DecodeString(signatureHex)
	if err != nil || len(signature) != ed25519.SignatureSize {
		return ErrInvalidInput
	}
	if !ed25519.Verify(key, []byte(TelemetryContext+payload), signature) {
		return ErrInvalidProof
	}
	var in telemetryPayload
	dec := json.NewDecoder(strings.NewReader(payload))
	dec.DisallowUnknownFields()
	if err := dec.Decode(&in); err != nil {
		return ErrInvalidInput
	}
	now := s.now().UTC()
	observed := time.UnixMilli(in.ObservedAtMs).UTC()
	if observed.Before(now.Add(-TelemetrySkew)) || observed.After(now.Add(TelemetrySkew)) {
		return ErrInvalidInput
	}
	if !validPercent(in.CPUPct) || !validPercent(in.MemPct) || in.CPUCores < 0 || in.CPUCores > 4096 ||
		in.TasksActive < 0 || in.TasksInProgress < 0 || utf8Len(in.Version) > maxVersionRunes {
		return ErrInvalidInput
	}
	report := store.Telemetry{
		ObservedAt: observed, ReceivedAt: now, Version: in.Version,
		CPUPct: in.CPUPct, MemPct: in.MemPct, CPUCores: in.CPUCores,
		Compute: in.Roles.Compute, Verify: in.Roles.Verify,
		TasksActive: in.TasksActive, TasksInProgress: in.TasksInProgress,
		TasksFinalized: in.TasksFinalized, TasksOther: in.TasksOther,
		ReputationScore: finiteOrNil(in.Reputation.Score), ReputationGood: finiteOrNil(in.Reputation.Good),
		ReputationBad:  finiteOrNil(in.Reputation.Bad),
		WorkerLamports: in.Earnings.WorkerLamports, VerifierLamports: in.Earnings.VerifierLamports,
		FragmentsSettled: in.Earnings.FragmentsSettled,
	}
	err = s.store.SetTelemetry(publicKeyHex, report)
	switch {
	case errors.Is(err, store.ErrNotFound):
		return ErrUnknownDevice
	case errors.Is(err, store.ErrStale):
		return ErrStaleReport
	}
	return err
}

// finiteOrNil keeps a reported number only when it is a usable value (not NaN or infinite).
func finiteOrNil(value *float64) *float64 {
	if value == nil || math.IsNaN(*value) || math.IsInf(*value, 0) {
		return nil
	}
	return value
}

func validPercent(value float64) bool {
	return !math.IsNaN(value) && value >= 0 && value <= 100.5
}

func utf8Len(text string) int {
	return len([]rune(text))
}
