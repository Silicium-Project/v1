// Package workloadspec holds the descriptors of the network's workloads. The data comes from
// contracts/workloads/*.json through tools/workloads/generate.mjs (generated.go); this file
// only declares the types. The API reads them and never edits them.
package workloadspec

// UnitKind is the work unit a workload pays for (sil-chain-core::units).
type UnitKind string

// The unit kinds known to the network. A new kind needs an on-chain rule before it can be used.
const (
	UnitBytesInput UnitKind = "bytes_input"
	UnitPixelsBand UnitKind = "pixels_band"
)

// Spec is one workload descriptor (see contracts/workloads/ for the meaning of each field).
type Spec struct {
	ID                    string
	Unit                  UnitKind
	Description           string
	LabelKey              string
	Params                []string
	MaxInputBytes         uint64
	MaxSide               uint32
	BlockRows             uint32
	MaxFragments          uint32
	TargetSize            uint64
	RewardPerUnit         uint64
	VerifierRewardPerUnit uint64
	Policy                string
	MinCheatFraction      float64
	TargetDetection       float64
	WallMs                uint64
	MemoryBytes           uint64
	OutputBytes           uint64
	DepositSupported      bool
}

// Find returns the descriptor of an identifier, if the network knows it.
func Find(id string) (Spec, bool) {
	for _, spec := range Workloads {
		if spec.ID == id {
			return spec, true
		}
	}
	return Spec{}, false
}
