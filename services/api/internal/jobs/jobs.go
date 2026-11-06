// Package jobs forwards what the user signed to a node and reports job state honestly.
//
// The API never signs for the user. The requester signs the JobSpec and the JobPublished
// event on their own side (wallet or desktop app, see contracts/); this service uploads
// the input, forwards the signed event, and keeps an index of "my jobs".
package jobs

import (
	"context"
	"encoding/json"
	"errors"
	"regexp"
	"strings"
	"time"
	"unicode/utf8"

	"sil/api/internal/nodeclient"
	"sil/api/internal/store"
)

// Limits enforced here (the HTTP layer also caps body sizes).
const (
	MaxEventBytes   = 64 * 1024
	MaxContentBytes = 64 << 20
	MaxJobsPerUser  = 10_000
	maxLabelRunes   = 100
)

var (
	ErrInvalidInput = errors.New("invalid input")
	ErrJobTaken     = errors.New("job already registered by another account")
	ErrTooMany      = errors.New("too many jobs")
	ErrNotRecorded  = errors.New("the node does not know this job after submission")
	ErrNotFound     = errors.New("job not found")
	ErrUnavailable  = nodeclient.ErrUnavailable
	ErrRejected     = nodeclient.ErrRejected
)

var jobID = regexp.MustCompile(`^[0-9a-f]{64}$`)

// Node is what the service needs from a node client.
type Node interface {
	PutContent(ctx context.Context, data []byte) (nodeclient.ContentReceipt, error)
	SubmitEvent(ctx context.Context, event []byte) (nodeclient.EventReceipt, error)
	Job(ctx context.Context, id string) (json.RawMessage, error)
}

// Service holds the job use cases.
type Service struct {
	store *store.Store
	node  Node
	now   func() time.Time
}

// New builds the service. now may be nil (wall clock).
func New(s *store.Store, node Node, now func() time.Time) *Service {
	if now == nil {
		now = time.Now
	}
	return &Service{store: s, node: node, now: now}
}

// UploadContent sends an input file to the node and returns its content id.
func (s *Service) UploadContent(ctx context.Context, data []byte) (nodeclient.ContentReceipt, error) {
	if len(data) == 0 || len(data) > MaxContentBytes {
		return nodeclient.ContentReceipt{}, ErrInvalidInput
	}
	return s.node.PutContent(ctx, data)
}

// Publish forwards a signed JobPublished event, then records the job for the user. The
// record is bookkeeping only; authority over a job is the requester signature.
func (s *Service) Publish(ctx context.Context, userID, id string, event []byte, label string) (store.Job, error) {
	label = strings.TrimSpace(label)
	if !jobID.MatchString(id) || len(event) == 0 || len(event) > MaxEventBytes ||
		utf8.RuneCountInString(label) > maxLabelRunes {
		return store.Job{}, ErrInvalidInput
	}
	if len(s.store.JobsOf(userID)) >= MaxJobsPerUser {
		return store.Job{}, ErrTooMany
	}
	if _, err := s.node.SubmitEvent(ctx, event); err != nil {
		return store.Job{}, err
	}
	// Do not index something the node does not recognise as a job.
	if _, err := s.node.Job(ctx, id); err != nil {
		if errors.Is(err, nodeclient.ErrNotFound) {
			return store.Job{}, ErrNotRecorded
		}
		return store.Job{}, err
	}
	job := store.Job{ID: id, UserID: userID, Label: label, CreatedAt: s.now().UTC()}
	if err := s.store.AddJob(job); err != nil {
		if errors.Is(err, store.ErrExists) {
			return store.Job{}, ErrJobTaken
		}
		return store.Job{}, err
	}
	return job, nil
}

// List returns the user jobs, newest first.
func (s *Service) List(userID string) []store.Job { return s.store.JobsOf(userID) }

// View is what the API shows for one job.
type View struct {
	Job    store.Job `json:"job"`
	Status Status    `json:"status"`
}

// Get returns a job of the user together with its honest status.
func (s *Service) Get(ctx context.Context, userID, id string) (View, error) {
	if !jobID.MatchString(id) {
		return View{}, ErrInvalidInput
	}
	job, err := s.store.JobOf(userID, id)
	if err != nil {
		return View{}, ErrNotFound
	}
	raw, err := s.node.Job(ctx, id)
	return View{Job: job, Status: Interpret(raw, err)}, nil
}
