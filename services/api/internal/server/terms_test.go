package server

import (
	"net/http"
	"testing"
)

func TestTermsAreSetByTheNetworkNotByTheUser(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "dana@example.com")

	status, out := e.call(t, "GET", "/v1/network/terms?workload=calcul/sha256&size=2048", token, nil)
	if status != http.StatusOK {
		t.Fatalf("terms: %d %v", status, out)
	}
	terms := out["terms"].(map[string]any)
	if terms["fragments"] != float64(1) || terms["reward_per_unit"] != float64(100) || terms["verifier_reward_per_unit"] != float64(10) {
		t.Fatalf("2 Ko : %v", terms)
	}
	if _, present := out["committee"]; present {
		t.Fatalf("le comité n'est plus donné aux jobs : %v", out)
	}
}

func TestTermsRefuseUnknownWorkloadsAndBadSizes(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "erin@example.com")
	for _, path := range []string{
		"/v1/network/terms?workload=ia/llm&size=10",
		"/v1/network/terms?workload=calcul/sha256&size=abc",
		"/v1/network/terms?workload=calcul/sha256&size=0",
		"/v1/network/terms?workload=rendu/raytracer&size=10",
	} {
		if status, _ := e.call(t, "GET", path, token, nil); status != http.StatusBadRequest {
			t.Fatalf("%s : 400 attendu, reçu %d", path, status)
		}
	}
}

func TestTermsNeedASession(t *testing.T) {
	e := newEnv(t, true)
	if status, _ := e.call(t, "GET", "/v1/network/terms?workload=calcul/sha256&size=10", "", nil); status != http.StatusUnauthorized {
		t.Fatalf("sans session : 401 attendu, reçu %d", status)
	}
}
