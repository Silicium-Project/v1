package server

import (
	"net/http"

	"sil/api/internal/httpx"
)

// networkStatus is the public status of the network: how many machines the node sees, by role.
// No session: it only counts, and it keeps no identifier or address.
func (s *Server) networkStatus(w http.ResponseWriter, r *http.Request) {
	status := s.status.Get(r.Context(), s.opts.Network)
	httpx.WriteJSON(w, http.StatusOK, status)
}
