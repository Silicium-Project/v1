package jobs

import (
	"encoding/json"
	"errors"

	"sil/api/internal/nodeclient"
)

// State values. "unknown" is deliberately distinct from every real state: when the node
// cannot be asked, the API must not claim that the job is running, failed or done.
const (
	StateUnknown   = "unknown"     // the node could not be asked or answered something unreadable
	StateNotOnNode = "not_on_node" // the node does not know this job (yet, or any more)
	StateRunning   = "running"
	StateFinalized = "finalized" // finalized in the ledger; payment is a separate matter
	StateCancelled = "cancelled"
	StateExpired   = "expired"
)

// Status is the honest status of a job.
type Status struct {
	State string `json:"state"`
	// Tasks counts fragments by the node own task status names.
	Tasks map[string]int `json:"tasks,omitempty"`
	// Note explains limits of the answer.
	Note string `json:"note,omitempty"`
}

const finalizedNote = "finalized in the ledger; settlement on chain is tracked separately and is not implied"

// Interpret turns a node answer (or error) into a Status.
func Interpret(raw json.RawMessage, err error) Status {
	switch {
	case errors.Is(err, nodeclient.ErrNotFound):
		return Status{State: StateNotOnNode, Note: "the node does not know this job"}
	case err != nil:
		return Status{State: StateUnknown, Note: "the node could not be reached; the job state is not known"}
	}
	var view struct {
		Status string `json:"status"`
		Tasks  []struct {
			Status string `json:"status"`
		} `json:"tasks"`
	}
	if json.Unmarshal(raw, &view) != nil {
		return Status{State: StateUnknown, Note: "unreadable answer from the node"}
	}
	tasks := map[string]int{}
	for _, task := range view.Tasks {
		tasks[task.Status]++
	}
	status := Status{Tasks: tasks}
	switch view.Status {
	case "Active":
		status.State = StateRunning
	case "Finalized":
		status.State, status.Note = StateFinalized, finalizedNote
	case "Cancelled":
		status.State = StateCancelled
	case "Expired":
		status.State = StateExpired
	default:
		return Status{State: StateUnknown, Note: "unrecognised job status from the node"}
	}
	return status
}
