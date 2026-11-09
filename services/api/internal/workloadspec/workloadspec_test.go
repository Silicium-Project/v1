package workloadspec

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

// descriptor is the part of contracts/workloads/*.json the API reads, with the JSON names.
type descriptor struct {
	ID    string `json:"id"`
	Unit  string `json:"unit"`
	Split struct {
		MaxFragments uint32 `json:"max_fragments"`
		TargetSize   uint64 `json:"target_size"`
	} `json:"split"`
	Terms struct {
		Reward         uint64 `json:"reward_per_unit"`
		VerifierReward uint64 `json:"verifier_reward_per_unit"`
	} `json:"terms"`
	Deposit string `json:"deposit"`
}

// The generated list must match the descriptor files it was produced from: a stale generated.go
// fails here even when nobody ran the generator check.
func TestGeneratedListMatchesTheDescriptors(t *testing.T) {
	files, err := filepath.Glob(filepath.Join("..", "..", "..", "..", "contracts", "workloads", "*.json"))
	if err != nil || len(files) == 0 {
		t.Fatalf("descripteurs introuvables : %v", err)
	}
	if len(files) != len(Workloads) {
		t.Fatalf("%d descripteurs, %d workloads générés : relancer tools/workloads/generate.mjs", len(files), len(Workloads))
	}
	for _, file := range files {
		raw, err := os.ReadFile(file)
		if err != nil {
			t.Fatal(err)
		}
		var d descriptor
		if err := json.Unmarshal(raw, &d); err != nil {
			t.Fatalf("%s : %v", file, err)
		}
		spec, ok := Find(d.ID)
		if !ok {
			t.Fatalf("%s : workload %q absent du code généré", file, d.ID)
		}
		if string(spec.Unit) != d.Unit || spec.MaxFragments != d.Split.MaxFragments || spec.TargetSize != d.Split.TargetSize {
			t.Fatalf("%s : découpage ou unité périmés : %+v", file, spec)
		}
		if spec.RewardPerUnit != d.Terms.Reward || spec.VerifierRewardPerUnit != d.Terms.VerifierReward {
			t.Fatalf("%s : tarifs périmés : %+v", file, spec)
		}
		if spec.DepositSupported != (d.Deposit == "supported") {
			t.Fatalf("%s : dépôt périmé : %+v", file, spec)
		}
	}
}

func TestUnknownIdentifierHasNoSpec(t *testing.T) {
	if _, ok := Find("ia/llm"); ok {
		t.Fatal("un identifiant inconnu n'a pas de descripteur")
	}
}
