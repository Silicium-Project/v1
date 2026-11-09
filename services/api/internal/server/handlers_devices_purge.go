package server

import "net/http"

// purgeDevice deletes a revoked device of the account for good. Active devices are refused: the
// user revokes them first, then deletes them.
func (s *Server) purgeDevice(w http.ResponseWriter, r *http.Request, userID, _ string) {
	if err := s.devices.Purge(userID, r.PathValue("id")); err != nil {
		s.fail(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
