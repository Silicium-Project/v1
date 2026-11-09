package server

import (
	"net/http"
)

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
