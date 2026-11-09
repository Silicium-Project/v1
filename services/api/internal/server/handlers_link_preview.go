package server

import (
	"net/http"

	"sil/api/internal/httpx"
)

// previewLinkCode tells the machine which account a link code belongs to, before the owner
// confirms. It needs no session: the code is the only input, and it is short-lived and single-use.
func (s *Server) previewLinkCode(w http.ResponseWriter, r *http.Request) {
	preview, err := s.devices.Preview(r.PathValue("code"))
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, preview)
}
