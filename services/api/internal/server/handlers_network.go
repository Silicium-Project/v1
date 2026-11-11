package server

import (
	"net/http"
	"strconv"

	"sil/api/internal/httpx"
	"sil/api/internal/policy"
)

// networkTerms gives the terms the network sets for a job: fragments, rewards, verifiers and
// leases. The user chooses none of them; the requester signs them and the wallet shows them.
// Query: workload, size (stored input in bytes), width and height (rendu/raytracer).
func (s *Server) networkTerms(w http.ResponseWriter, r *http.Request, _, _ string) {
	query := r.URL.Query()
	size, err := strconv.ParseUint(query.Get("size"), 10, 64)
	if err != nil {
		httpx.WriteError(w, http.StatusBadRequest, "invalid_input", "size: entier attendu")
		return
	}
	width, _ := strconv.ParseUint(query.Get("width"), 10, 32)
	height, _ := strconv.ParseUint(query.Get("height"), 10, 32)
	terms, err := policy.For(query.Get("workload"), size, uint32(width), uint32(height))
	if err != nil {
		httpx.WriteError(w, http.StatusBadRequest, "invalid_input", err.Error())
		return
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{
		"terms":    terms,
		"payments": s.opts.Payments,
	})
}
