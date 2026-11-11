package server

import (
	"net/http"
)

// jobInput sends the input content of a job of the user, as the node stores it (raw bytes).
func (s *Server) jobInput(w http.ResponseWriter, r *http.Request, userID, _ string) {
	ctx, cancel := requestContext(r)
	defer cancel()
	body, err := s.jobs.Input(ctx, userID, r.PathValue("id"))
	if err != nil {
		s.fail(w, err)
		return
	}
	w.Header().Set("Content-Type", "application/octet-stream")
	w.Header().Set("Cache-Control", "no-store")
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(body)
}

// result sends the finished result of a job of the user: a PNG picture for a render job, a text
// file with the raw fragment results otherwise.
func (s *Server) result(w http.ResponseWriter, r *http.Request, userID, _ string) {
	ctx, cancel := requestContext(r)
	defer cancel()
	id := r.PathValue("id")
	result, err := s.jobs.Result(ctx, userID, id)
	if err != nil {
		s.fail(w, err)
		return
	}
	w.Header().Set("Content-Type", result.ContentType)
	w.Header().Set("Content-Disposition", `attachment; filename="`+result.Filename+`"`)
	w.Header().Set("Cache-Control", "no-store")
	w.WriteHeader(http.StatusOK)
	_, _ = w.Write(result.Body)
}
