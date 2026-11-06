package server

import (
	"net/http"
	"time"

	"sil/api/internal/httpx"
	"sil/api/internal/store"
)

type deviceView struct {
	ID        string     `json:"id"`
	PublicKey string     `json:"public_key"`
	Label     string     `json:"label"`
	LinkedAt  time.Time  `json:"linked_at"`
	RevokedAt *time.Time `json:"revoked_at,omitempty"`
}

func viewOf(d store.Device) deviceView {
	return deviceView{ID: d.ID, PublicKey: d.PublicKey, Label: d.Label, LinkedAt: d.LinkedAt, RevokedAt: d.RevokedAt}
}

func (s *Server) newLinkCode(w http.ResponseWriter, _ *http.Request, userID, _ string) {
	code, expires, err := s.devices.NewCode(userID)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, map[string]any{"code": code, "expires_at": expires})
}

type linkRequest struct {
	Code      string `json:"code"`
	PublicKey string `json:"public_key"`
	Signature string `json:"signature"`
	Label     string `json:"label"`
}

// linkDevice is called by the machine itself, not by a logged-in user: the proof of key
// ownership plus the one-time code are the authentication.
func (s *Server) linkDevice(w http.ResponseWriter, r *http.Request) {
	var in linkRequest
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	device, err := s.devices.Link(in.Code, in.PublicKey, in.Signature, in.Label)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, viewOf(device))
}

func (s *Server) listDevices(w http.ResponseWriter, _ *http.Request, userID, _ string) {
	views := []deviceView{}
	for _, d := range s.devices.List(userID) {
		views = append(views, viewOf(d))
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{"devices": views})
}

func (s *Server) revokeDevice(w http.ResponseWriter, r *http.Request, userID, _ string) {
	if err := s.devices.Revoke(userID, r.PathValue("id")); err != nil {
		s.fail(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
