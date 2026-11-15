package network

import (
	"context"
	"encoding/json"
	"errors"
	"testing"
	"time"
)

var t0 = time.Date(2026, 11, 7, 16, 0, 0, 0, time.UTC)

// The node's own report: compute only.
const selfReport = `{"roles":{"compute":true,"verify":false}}`

// Three machines heard from: one recent worker-verifier, one silent for two minutes (not
// online), one recent verifier. No key or address appears in the answer.
const peersJSON = `[
 {"peer":"12D3KooWaaaa","roles":["compute","verify"],"seen_ms_ago":5000,"load":0,"workloads":[]},
 {"peer":"12D3KooWbbbb","roles":["compute","verify"],"seen_ms_ago":120000,"load":0,"workloads":[]},
 {"peer":"12D3KooWcccc","roles":["verify"],"seen_ms_ago":20000,"load":0,"workloads":[]}
]`

func TestSummarizeCountsOnlyMachinesHeardRecently(t *testing.T) {
	status, err := Summarize([]byte(peersJSON), []byte(selfReport), t0)
	if err != nil {
		t.Fatal(err)
	}
	if status.Online != 3 {
		t.Fatalf("online = %d, want 3 (node + two recent machines)", status.Online)
	}
	if status.ByRole["compute"] != 2 || status.ByRole["verify"] != 2 {
		t.Fatalf("by role = %v", status.ByRole)
	}
	if !status.NodeReachable {
		t.Fatal("the answering node is reachable")
	}
}

func TestUnreachableNodeGivesNoCounts(t *testing.T) {
	status := Unreachable(t0)
	if status.NodeReachable || status.Online != 0 {
		t.Fatalf("unreachable = %+v", status)
	}
}

type fakeSource struct {
	peers    json.RawMessage
	err      error
	peerAsks int
}

func (f *fakeSource) Peers(context.Context) (json.RawMessage, error) {
	f.peerAsks++
	return f.peers, f.err
}

func (f *fakeSource) Worker(context.Context) (json.RawMessage, error) {
	return json.RawMessage(selfReport), nil
}

func TestCacheAsksTheNodeOnlyWhenStale(t *testing.T) {
	clock := t0
	cache := NewCache(10*time.Second, func() time.Time { return clock })
	src := &fakeSource{peers: json.RawMessage(peersJSON)}
	cache.Get(context.Background(), src)
	cache.Get(context.Background(), src)
	if src.peerAsks != 1 {
		t.Fatalf("asked %d times within the ttl, want 1", src.peerAsks)
	}
	clock = t0.Add(11 * time.Second)
	cache.Get(context.Background(), src)
	if src.peerAsks != 2 {
		t.Fatalf("asked %d times after the ttl, want 2", src.peerAsks)
	}
}

func TestAFailingNodeIsReportedAndNotCached(t *testing.T) {
	cache := NewCache(10*time.Second, func() time.Time { return t0 })
	src := &fakeSource{err: errors.New("down")}
	if status := cache.Get(context.Background(), src); status.NodeReachable {
		t.Fatal("a failing node must read as unreachable")
	}
	src.err = nil
	src.peers = json.RawMessage(peersJSON)
	if status := cache.Get(context.Background(), src); !status.NodeReachable {
		t.Fatal("the next answer must be asked again, not the failure")
	}
}

// Only the machines that can read the chain (role "claim") are counted as able to claim.
const claimPeersJSON = `[
 {"peer":"12D3KooWdddd","roles":["compute","verify","claim"],"seen_ms_ago":5000,"load":0,"workloads":[]},
 {"peer":"12D3KooWeeee","roles":["compute","verify"],"seen_ms_ago":5000,"load":0,"workloads":[]}
]`

func TestSummarizeCountsClaimCapableMachines(t *testing.T) {
	status, err := Summarize([]byte(claimPeersJSON), []byte(`{"roles":{"compute":true,"verify":false,"claim":true}}`), t0)
	if err != nil {
		t.Fatal(err)
	}
	if status.ByRole["compute"] != 3 || status.ByRole["claim"] != 2 {
		t.Fatalf("by role = %v, want compute 3 and claim 2 (node + one peer with claim)", status.ByRole)
	}
}
