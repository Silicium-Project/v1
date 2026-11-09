package jobs

import (
	"encoding/json"
	"testing"
)

// The node answer for a job of two fragments: fragment 1 is still being verified, fragment 0 is
// final. Listed out of order on purpose.
const progressJSON = `{
  "status": "Active",
  "tasks": [
    {
      "task": {"job": "ab", "fragment": 1},
      "status": "Completed",
      "attempts": [{"verifiers": [{"verdict": true}, {"verdict": null}]}],
      "finalized": null
    },
    {
      "task": {"job": "ab", "fragment": 0},
      "status": "Finalized",
      "attempts": [{"verifiers": []}, {"verifiers": [{"verdict": true}, {"verdict": true}]}],
      "finalized": {"attempt": 2}
    }
  ]
}`

func TestProgressListsEachFragmentInOrder(t *testing.T) {
	status := Interpret(json.RawMessage(progressJSON), nil)
	if status.State != StateRunning {
		t.Fatalf("state %s", status.State)
	}
	if len(status.Fragments) != 2 {
		t.Fatalf("fragments: %+v", status.Fragments)
	}
	first, second := status.Fragments[0], status.Fragments[1]
	if first.Index != 0 || !first.Finalized || first.Attempts != 2 || first.Approvals != 2 || first.Status != "Finalized" {
		t.Fatalf("fragment 0: %+v", first)
	}
	if second.Index != 1 || second.Finalized || second.Attempts != 1 || second.Approvals != 1 || second.Status != "Completed" {
		t.Fatalf("fragment 1: %+v", second)
	}
}

func TestUnreadableProgressIsUnknownNotInvented(t *testing.T) {
	status := Interpret(json.RawMessage(`{"status":"Active","tasks":"nope"}`), nil)
	if status.State != StateUnknown || status.Fragments != nil {
		t.Fatalf("status %+v", status)
	}
}
