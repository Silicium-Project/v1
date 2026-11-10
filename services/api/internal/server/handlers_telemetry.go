package server

import (
	"net/http"

	"sil/api/internal/httpx"
)

type telemetryRequest struct {
	DeviceKey string `json:"device_key"`
	Payload   string `json:"payload"`
	Signature string `json:"signature"`
}

// reportTelemetry is called by a machine, not by a logged-in user: the signature made with the
// device key is the authentication. Only an active device linked to an account is accepted.
func (s *Server) reportTelemetry(w http.ResponseWriter, r *http.Request) {
	var in telemetryRequest
	if err := httpx.ReadJSON(w, r, &in, 8192); err != nil {
		badBody(w, err)
		return
	}
	if err := s.devices.ReportTelemetry(in.DeviceKey, in.Payload, in.Signature); err != nil {
		s.fail(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
