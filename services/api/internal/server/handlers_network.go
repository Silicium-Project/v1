package server

import (
	"net/http"

	"sil/api/internal/httpx"
)

// committee lists the coordinators a new job is given to. It is operator configuration for now
// (SIL_API_COMMITTEE): choosing it from the network would accept any node that claims the role.
func (s *Server) committee(w http.ResponseWriter, _ *http.Request, _, _ string) {
	keys := s.opts.Committee
	threshold := 0
	if len(keys) > 0 {
		threshold = len(keys)/2 + 1 // strict majority, as sil-ledger requires
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{
		"committee": keys,
		"threshold": threshold,
		"workloads": []string{"calcul/sha256", "rendu/blocs"},
		"payments":  false, // the escrow adapter is not built: no job is paid yet
	})
}
