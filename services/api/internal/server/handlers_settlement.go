package server

import (
	"io"
	"net/http"

	"sil/api/internal/httpx"
	"sil/api/internal/jobs"
)

// submitSettlement relays a settlement signed by the approvers of a job. Anyone may send one: the
// signatures are the proof and the chain checks them; the node that receives it pays the fees.
// No account is needed, so each client address is rate limited (see Handler).
func (s *Server) submitSettlement(w http.ResponseWriter, r *http.Request) {
	body, err := io.ReadAll(http.MaxBytesReader(w, r.Body, jobs.MaxEventBytes))
	if err != nil {
		httpx.WriteError(w, http.StatusRequestEntityTooLarge, "too_large", "settlement too large")
		return
	}
	ctx, cancel := requestContext(r)
	defer cancel()
	answer, err := s.jobs.Settle(ctx, body)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusAccepted, answer)
}
