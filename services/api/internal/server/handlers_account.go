package server

import (
	"net/http"
	"strings"
	"time"

	"sil/api/internal/httpx"
)

type credentials struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

func (s *Server) register(w http.ResponseWriter, r *http.Request) {
	if !s.opts.AllowRegistration {
		httpx.WriteError(w, http.StatusForbidden, "registration_closed", "registration is closed")
		return
	}
	var in credentials
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	user, err := s.auth.Register(in.Email, in.Password)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, map[string]string{"id": user.ID, "email": user.Email})
}

func (s *Server) login(w http.ResponseWriter, r *http.Request) {
	var in credentials
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	// Slow, per-account limit: stops password guessing spread over many addresses.
	if !s.loginEmail.Allow(strings.ToLower(strings.TrimSpace(in.Email)), time.Now()) {
		w.Header().Set("Retry-After", "180")
		httpx.WriteError(w, http.StatusTooManyRequests, "rate_limited", "too many login attempts")
		return
	}
	token, expires, err := s.auth.Login(in.Email, in.Password)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{"token": token, "expires_at": expires})
}

func (s *Server) logout(w http.ResponseWriter, _ *http.Request, _, token string) {
	if err := s.auth.Logout(token); err != nil {
		s.fail(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}
