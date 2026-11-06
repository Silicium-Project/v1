package jobs

import (
	"context"
	"encoding/json"
	"errors"
	"strings"
	"testing"

	"sil/api/internal/nodeclient"
	"sil/api/internal/store"
)

type fakeNode struct {
	submitErr, jobErr error
	job               string
	submitted         [][]byte
}

func (f *fakeNode) PutContent(_ context.Context, d []byte) (nodeclient.ContentReceipt, error) {
	return nodeclient.ContentReceipt{ContentID: "cid", Size: len(d)}, nil
}

func (f *fakeNode) SubmitEvent(_ context.Context, e []byte) (nodeclient.EventReceipt, error) {
	if f.submitErr == nil {
		f.submitted = append(f.submitted, e)
	}
	return nodeclient.EventReceipt{Accepted: 1}, f.submitErr
}

func (f *fakeNode) Job(context.Context, string) (json.RawMessage, error) {
	return json.RawMessage(f.job), f.jobErr
}

var id = strings.Repeat("ab", 32)

func setup() (*Service, *fakeNode) {
	st, _ := store.Open("")
	node := &fakeNode{job: `{"status":"Active","tasks":[{"status":"Open"},{"status":"Open"}]}`}
	return New(st, node, nil), node
}

func TestPublishForwardsThenRecords(t *testing.T) {
	svc, node := setup()
	job, err := svc.Publish(context.Background(), "u", id, []byte("event"), " my job ")
	if err != nil {
		t.Fatal(err)
	}
	if job.Label != "my job" || len(node.submitted) != 1 || len(svc.List("u")) != 1 {
		t.Fatalf("%+v %d", job, len(node.submitted))
	}
}

func TestNothingIsRecordedWhenTheNodeFails(t *testing.T) {
	svc, node := setup()
	node.submitErr = nodeclient.ErrUnavailable
	if _, err := svc.Publish(context.Background(), "u", id, []byte("e"), ""); !errors.Is(err, ErrUnavailable) {
		t.Fatalf("got %v", err)
	}
	node.submitErr = nodeclient.ErrRejected
	if _, err := svc.Publish(context.Background(), "u", id, []byte("e"), ""); !errors.Is(err, ErrRejected) {
		t.Fatalf("got %v", err)
	}
	if len(svc.List("u")) != 0 {
		t.Fatal("job recorded although the node failed")
	}
}

func TestEventThatDoesNotCreateTheJobIsNotIndexed(t *testing.T) {
	svc, node := setup()
	node.jobErr = nodeclient.ErrNotFound
	if _, err := svc.Publish(context.Background(), "u", id, []byte("e"), ""); !errors.Is(err, ErrNotRecorded) {
		t.Fatalf("got %v", err)
	}
	if len(svc.List("u")) != 0 {
		t.Fatal("indexed")
	}
}

func TestInputValidation(t *testing.T) {
	svc, _ := setup()
	ctx := context.Background()
	for name, call := range map[string]func() error{
		"upper-case id": func() error { _, e := svc.Publish(ctx, "u", strings.ToUpper(id), []byte("e"), ""); return e },
		"short id":      func() error { _, e := svc.Publish(ctx, "u", "ab", []byte("e"), ""); return e },
		"empty event":   func() error { _, e := svc.Publish(ctx, "u", id, nil, ""); return e },
		"huge event":    func() error { _, e := svc.Publish(ctx, "u", id, make([]byte, MaxEventBytes+1), ""); return e },
		"long label":    func() error { _, e := svc.Publish(ctx, "u", id, []byte("e"), strings.Repeat("x", 101)); return e },
		"empty content": func() error { _, e := svc.UploadContent(ctx, nil); return e },
	} {
		if err := call(); !errors.Is(err, ErrInvalidInput) {
			t.Errorf("%s: got %v", name, err)
		}
	}
}

func TestAnotherAccountCannotTakeOrReadAJob(t *testing.T) {
	svc, _ := setup()
	ctx := context.Background()
	if _, err := svc.Publish(ctx, "u", id, []byte("e"), ""); err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Publish(ctx, "v", id, []byte("e"), ""); !errors.Is(err, ErrJobTaken) {
		t.Fatalf("take: %v", err)
	}
	if _, err := svc.Get(ctx, "v", id); !errors.Is(err, ErrNotFound) {
		t.Fatalf("read: %v", err)
	}
}

func TestStatusIsHonest(t *testing.T) {
	cases := []struct {
		name string
		raw  string
		err  error
		want string
	}{
		{"active", `{"status":"Active","tasks":[]}`, nil, StateRunning},
		{"finalized", `{"status":"Finalized","tasks":[]}`, nil, StateFinalized},
		{"cancelled", `{"status":"Cancelled"}`, nil, StateCancelled},
		{"expired", `{"status":"Expired"}`, nil, StateExpired},
		{"node down", ``, nodeclient.ErrUnavailable, StateUnknown},
		{"not on node", ``, nodeclient.ErrNotFound, StateNotOnNode},
		{"garbage", `<html>`, nil, StateUnknown},
		{"new status we do not know", `{"status":"Paid"}`, nil, StateUnknown},
	}
	for _, c := range cases {
		if got := Interpret(json.RawMessage(c.raw), c.err); got.State != c.want {
			t.Errorf("%s: got %q, want %q", c.name, got.State, c.want)
		}
	}
	if s := Interpret(json.RawMessage(`{"status":"Finalized"}`), nil); !strings.Contains(s.Note, "not implied") {
		t.Errorf("finalized must say payment is separate: %q", s.Note)
	}
}

func TestGetShowsUnknownWhenTheNodeIsDown(t *testing.T) {
	svc, node := setup()
	ctx := context.Background()
	_, _ = svc.Publish(ctx, "u", id, []byte("e"), "")
	node.jobErr = nodeclient.ErrUnavailable
	view, err := svc.Get(ctx, "u", id)
	if err != nil || view.Status.State != StateUnknown {
		t.Fatalf("%+v %v", view, err)
	}
}
