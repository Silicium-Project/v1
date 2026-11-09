package server

import (
	"encoding/base64"
	"net/http"

	"sil/api/internal/httpx"
	"sil/api/internal/store"
)

func (s *Server) uploadContent(w http.ResponseWriter, r *http.Request, _, _ string) {
	data, err := httpx.ReadBody(w, r, maxContentBody)
	if err != nil {
		badBody(w, err)
		return
	}
	ctx, cancel := requestContext(r)
	defer cancel()
	receipt, err := s.jobs.UploadContent(ctx, data)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, receipt)
}

type publishRequest struct {
	JobID string `json:"job_id"`
	// Event is the signed JobPublished event (canonical CBOR), base64 standard encoding.
	Event string `json:"event"`
	Label string `json:"label"`
}

type jobSummary struct {
	ID        string `json:"id"`
	Label     string `json:"label"`
	CreatedAt string `json:"created_at"`
}

func summary(j store.Job) jobSummary {
	return jobSummary{ID: j.ID, Label: j.Label, CreatedAt: j.CreatedAt.Format("2006-01-02T15:04:05Z")}
}

func (s *Server) publishJob(w http.ResponseWriter, r *http.Request, userID, _ string) {
	var in publishRequest
	if err := httpx.ReadJSON(w, r, &in, maxJSONBody); err != nil {
		badBody(w, err)
		return
	}
	event, err := base64.StdEncoding.DecodeString(in.Event)
	if err != nil {
		httpx.WriteError(w, http.StatusBadRequest, "invalid_input", "event is not valid base64")
		return
	}
	ctx, cancel := requestContext(r)
	defer cancel()
	job, err := s.jobs.Publish(ctx, userID, in.JobID, event, in.Label)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, summary(job))
}

type eventRequest struct {
	// Event is a signed event of the job (the cancellation), canonical CBOR, base64 standard.
	Event string `json:"event"`
}

func (s *Server) jobEvent(w http.ResponseWriter, r *http.Request, userID, _ string) {
	var in eventRequest
	if err := httpx.ReadJSON(w, r, &in, maxJSONBody); err != nil {
		badBody(w, err)
		return
	}
	event, err := base64.StdEncoding.DecodeString(in.Event)
	if err != nil {
		httpx.WriteError(w, http.StatusBadRequest, "invalid_input", "event is not valid base64")
		return
	}
	ctx, cancel := requestContext(r)
	defer cancel()
	receipt, err := s.jobs.Event(ctx, userID, r.PathValue("id"), event)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusAccepted, receipt)
}

func (s *Server) listJobs(w http.ResponseWriter, _ *http.Request, userID, _ string) {
	list := []jobSummary{}
	for _, j := range s.jobs.List(userID) {
		list = append(list, summary(j))
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{"jobs": list})
}

func (s *Server) getJob(w http.ResponseWriter, r *http.Request, userID, _ string) {
	ctx, cancel := requestContext(r)
	defer cancel()
	view, err := s.jobs.Get(ctx, userID, r.PathValue("id"))
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{
		"job": summary(view.Job), "status": view.Status,
	})
}
