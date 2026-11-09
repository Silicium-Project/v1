package policy

import "testing"

const (
	sha256ID = "calcul/sha256"
	renderID = "rendu/raytracer"
)

func TestSha256FragmentsFollowTheInputSize(t *testing.T) {
	small, err := For(sha256ID, 2048, 0, 0)
	if err != nil || small.Fragments != 1 {
		t.Fatalf("2 Ko : un fragment attendu, %+v, %v", small, err)
	}
	big, _ := For(sha256ID, 8<<20, 0, 0)
	if big.Fragments != 16 {
		t.Fatalf("8 Mo : plafond de 16 fragments, reçu %d", big.Fragments)
	}
	if big.RewardPerUnit != 100 || big.VerifierRewardPerUnit != 10 {
		t.Fatalf("tarifs : %+v", big)
	}
}

func TestNeverMoreFragmentsThanBytes(t *testing.T) {
	terms, _ := For(sha256ID, 3, 0, 0)
	if terms.Fragments != 1 {
		t.Fatalf("3 octets : %d fragments, 1 attendu", terms.Fragments)
	}
}

func TestRenderFragmentsFollowTheImageHeight(t *testing.T) {
	terms, err := For(renderID, 0, 800, 600)
	if err != nil || terms.Fragments < 1 || terms.Fragments > 16 {
		t.Fatalf("%+v, %v", terms, err)
	}
	if terms.RewardPerUnit != 1 || terms.VerifierRewardPerUnit != 1 {
		t.Fatalf("tarifs au pixel : %+v", terms)
	}
	if _, err := For(renderID, 0, 0, 600); err == nil {
		t.Fatal("image sans largeur : refusée attendue")
	}
}

func TestEveryJobGetsTheSameLeaseAndExpiry(t *testing.T) {
	terms, _ := For(sha256ID, 100, 0, 0)
	if terms.LeaseMs != 60_000 || terms.MaxAttempts != 3 || terms.VerificationsRequired != 1 || terms.ExpiresInMs != 86_400_000 {
		t.Fatalf("%+v", terms)
	}
}

func TestUnknownWorkloadIsRefused(t *testing.T) {
	if _, err := For("ia/llm", 10, 0, 0); err != ErrUnknownWorkload {
		t.Fatalf("erreur attendue : %v", err)
	}
}

func TestWorkloadIDsListEveryDescriptor(t *testing.T) {
	ids := WorkloadIDs()
	if len(ids) != 2 || ids[0] != sha256ID || ids[1] != renderID {
		t.Fatalf("identifiants : %v", ids)
	}
}
