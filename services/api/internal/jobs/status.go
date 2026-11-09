package jobs

import (
	"encoding/json"
	"errors"
	"sort"

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
	// Workload is the workload id of the job, as the node knows it.
	Workload string `json:"workload,omitempty"`
	// Image is true when the job produces a picture (the render workload).
	Image bool `json:"image"`
	// Fragments is the progress of each fragment, by index.
	Fragments []FragmentStatus `json:"fragments,omitempty"`
	// RequesterHead is the last event of the requester chain: the web builds the next event
	// (a cancellation) from it. Absent when the node does not know the job.
	RequesterHead *RequesterHead `json:"requester_head,omitempty"`
	// Note explains limits of the answer.
	Note string `json:"note,omitempty"`
}

// RequesterHead is the sequence number and the identifier (hex) of a requester chain head.
type RequesterHead struct {
	Sequence uint64 `json:"sequence"`
	Hash     string `json:"hash"`
}

// FragmentStatus is the progress of one fragment of a job, as the node sees it.
type FragmentStatus struct {
	Index int `json:"index"`
	// Status is the node task status name (Open, Claimed, Completed, Verified, Finalized...).
	Status string `json:"status"`
	// Attempts is how many attempts were accepted so far.
	Attempts int `json:"attempts"`
	// Approvals counts the verifiers who approved the latest attempt.
	Approvals int `json:"approvals"`
	// Finalized is true once the fragment has a final result in the ledger.
	Finalized bool `json:"finalized"`
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
		Status        string         `json:"status"`
		Workload      string         `json:"workload"`
		Image         bool           `json:"image"`
		RequesterHead *RequesterHead `json:"requester_head"`
		Tasks         []struct {
			Task struct {
				Fragment int `json:"fragment"`
			} `json:"task"`
			Status   string `json:"status"`
			Attempts []struct {
				Verifiers []struct {
					Verdict *bool `json:"verdict"`
				} `json:"verifiers"`
			} `json:"attempts"`
			Finalized *struct{} `json:"finalized"`
		} `json:"tasks"`
	}
	if json.Unmarshal(raw, &view) != nil {
		return Status{State: StateUnknown, Note: "unreadable answer from the node"}
	}
	tasks := map[string]int{}
	fragments := make([]FragmentStatus, 0, len(view.Tasks))
	for _, task := range view.Tasks {
		tasks[task.Status]++
		fragment := FragmentStatus{
			Index:     task.Task.Fragment,
			Status:    task.Status,
			Attempts:  len(task.Attempts),
			Finalized: task.Finalized != nil,
		}
		if n := len(task.Attempts); n > 0 {
			for _, verifier := range task.Attempts[n-1].Verifiers {
				if verifier.Verdict != nil && *verifier.Verdict {
					fragment.Approvals++
				}
			}
		}
		fragments = append(fragments, fragment)
	}
	sort.Slice(fragments, func(i, j int) bool { return fragments[i].Index < fragments[j].Index })
	status := Status{
		Tasks:         tasks,
		Fragments:     fragments,
		Workload:      view.Workload,
		Image:         view.Image,
		RequesterHead: view.RequesterHead,
	}
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
