// Package network summarises the machines a node sees, for the public status page. It keeps
// counts only: no peer identifier, address or key leaves the API.
package network

import (
	"context"
	"encoding/json"
	"time"
)

// Source gives what the node knows of the network: its own report (GET /v1/worker) and the
// machines it has heard from (GET /v1/peers).
type Source interface {
	Peers(ctx context.Context) (json.RawMessage, error)
	Worker(ctx context.Context) (json.RawMessage, error)
}

// OnlineWindow is how recently a machine must have been heard from to count as online.
const OnlineWindow = 60 * time.Second

type peer struct {
	Roles     []string `json:"roles"`
	SeenMsAgo uint64   `json:"seen_ms_ago"`
}

type self struct {
	Roles struct {
		Compute bool `json:"compute"`
		Verify  bool `json:"verify"`
		Claim   bool `json:"claim"`
	} `json:"roles"`
}

// Status is the public summary of the network, as one node sees it.
type Status struct {
	Online        int            `json:"online"`
	ByRole        map[string]int `json:"by_role"`
	NodeReachable bool           `json:"node_reachable"`
	UpdatedAt     time.Time      `json:"updated_at"`
}

// Summarize counts the node itself and every machine heard from within OnlineWindow.
func Summarize(peers, report json.RawMessage, now time.Time) (Status, error) {
	var heard []peer
	if err := json.Unmarshal(peers, &heard); err != nil {
		return Status{}, err
	}
	var me self
	if err := json.Unmarshal(report, &me); err != nil {
		return Status{}, err
	}
	status := Status{
		ByRole:        map[string]int{"compute": 0, "verify": 0, "claim": 0},
		NodeReachable: true,
		UpdatedAt:     now.UTC(),
	}
	// The node answering is online; its own roles count too.
	status.Online = 1
	count := func(compute, verify, claim bool) {
		if compute {
			status.ByRole["compute"]++
		}
		if verify {
			status.ByRole["verify"]++
		}
		if claim {
			status.ByRole["claim"]++
		}
	}
	count(me.Roles.Compute, me.Roles.Verify, me.Roles.Claim)
	for _, machine := range heard {
		if time.Duration(machine.SeenMsAgo)*time.Millisecond > OnlineWindow {
			continue
		}
		status.Online++
		has := func(role string) bool {
			for _, r := range machine.Roles {
				if r == role {
					return true
				}
			}
			return false
		}
		count(has("compute"), has("verify"), has("claim"))
	}
	return status, nil
}

// Unreachable is the answer when the node cannot be asked: the page says so, with no counts.
func Unreachable(now time.Time) Status {
	return Status{
		ByRole:        map[string]int{"compute": 0, "verify": 0, "claim": 0},
		NodeReachable: false,
		UpdatedAt:     now.UTC(),
	}
}
