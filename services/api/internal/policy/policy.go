// Package policy decides how a job runs: how many fragments, what each unit pays, how many
// verifiers and how long a job lives. The user only chooses a workload and sends a file; the
// network sets the rest. The requester signs these terms, so the wallet shows them.
//
// Everything that depends on a workload comes from its descriptor (workloadspec). This file only
// knows the unit kinds and the network-wide rules that apply to every job.
package policy

import (
	"errors"
	"time"

	"sil/api/internal/workloadspec"
)

// Terms are the values the network gives a job. Rewards are in lamports per unit.
type Terms struct {
	Fragments             int    `json:"fragments"`
	RewardPerUnit         uint64 `json:"reward_per_unit"`
	VerifierRewardPerUnit uint64 `json:"verifier_reward_per_unit"`
	VerificationsRequired int    `json:"verifications_required"`
	LeaseMs               uint64 `json:"lease_ms"`
	MaxAttempts           int    `json:"max_attempts"`
	ExpiresInMs           uint64 `json:"expires_in_ms"`
}

// Network-wide rules: the same for every workload.
const (
	leaseMs     = 60_000
	maxAttempts = 3
	expiresIn   = 24 * time.Hour
)

// ErrUnknownWorkload means the network has no descriptor for this workload.
var ErrUnknownWorkload = errors.New("workload inconnu")

// WorkloadIDs lists the workloads the network runs, in descriptor order.
func WorkloadIDs() []string {
	ids := make([]string, 0, len(workloadspec.Workloads))
	for _, spec := range workloadspec.Workloads {
		ids = append(ids, spec.ID)
	}
	return ids
}

// For returns the terms of a job. inputSize is the stored input in bytes (for an image workload,
// the size header included); width and height are the image size in pixels.
func For(workload string, inputSize uint64, width, height uint32) (Terms, error) {
	spec, ok := workloadspec.Find(workload)
	if !ok {
		return Terms{}, ErrUnknownWorkload
	}
	measure, err := measureOf(spec, inputSize, width, height)
	if err != nil {
		return Terms{}, err
	}
	return Terms{
		Fragments:             fragmentsFor(spec, measure),
		RewardPerUnit:         spec.RewardPerUnit,
		VerifierRewardPerUnit: spec.VerifierRewardPerUnit,
		VerificationsRequired: 1,
		LeaseMs:               leaseMs,
		MaxAttempts:           maxAttempts,
		ExpiresInMs:           uint64(expiresIn / time.Millisecond),
	}, nil
}

// measureOf is the size the fragments are cut from, in the unit of the descriptor's split:
// bytes of input for a bytes unit, blocks of rows for a pixel band.
func measureOf(spec workloadspec.Spec, inputSize uint64, width, height uint32) (uint64, error) {
	switch spec.Unit {
	case workloadspec.UnitBytesInput:
		if inputSize == 0 {
			return 0, errors.New("entrée vide")
		}
		return inputSize, nil
	case workloadspec.UnitPixelsBand:
		if width == 0 || height == 0 {
			return 0, errors.New("taille d'image requise")
		}
		return ceilDiv(uint64(height), uint64(spec.BlockRows)), nil
	}
	return 0, ErrUnknownWorkload
}

// fragmentsFor cuts the measure into about target_size units per fragment, capped by the
// descriptor's max_fragments and by the measure itself (a fragment is never empty).
func fragmentsFor(spec workloadspec.Spec, measure uint64) int {
	return int(min(uint64(spec.MaxFragments), ceilDiv(measure, spec.TargetSize), measure))
}

func ceilDiv(a, b uint64) uint64 {
	return (a + b - 1) / b
}
