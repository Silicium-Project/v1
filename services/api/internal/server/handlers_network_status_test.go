package server

import "testing"

// The status page needs no session. Without a node to ask, it says so and gives no counts.
func TestNetworkStatusIsPublicAndSaysWhenTheNodeIsUnreachable(t *testing.T) {
	e := newEnv(t, true)
	status, out := e.call(t, "GET", "/v1/network/status", "", nil)
	if status != 200 {
		t.Fatalf("status: %d %v", status, out)
	}
	if out["node_reachable"] != false {
		t.Fatalf("node_reachable: %v", out)
	}
	if out["online"] != float64(0) {
		t.Fatalf("online without a node: %v", out)
	}
}
