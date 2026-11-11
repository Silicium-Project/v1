package server

import (
	"net/http"

	"sil/api/internal/httpx"
)

// maxOriginsBody bounds the JSON of the origins of one job (2 Mo of text plus JSON overhead).
const maxOriginsBody = 3 << 20

type originsRequest struct {
	Origins []string `json:"origins"`
}

// putOrigins records where each passage of an embedding job comes from. Owner only.
func (s *Server) putOrigins(w http.ResponseWriter, r *http.Request, userID, _ string) {
	var in originsRequest
	if err := httpx.ReadJSON(w, r, &in, maxOriginsBody); err != nil {
		badBody(w, err)
		return
	}
	if err := s.jobs.SetOrigins(userID, r.PathValue("id"), in.Origins); err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]int{"count": len(in.Origins)})
}

// getOrigins returns the origins of a job of the user.
func (s *Server) getOrigins(w http.ResponseWriter, r *http.Request, userID, _ string) {
	origins, err := s.jobs.Origins(userID, r.PathValue("id"))
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, originsRequest{Origins: origins})
}
