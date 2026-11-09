package server

import (
	"encoding/base64"
	"net/http"
	"time"

	"sil/api/internal/httpx"
)

func (s *Server) profile(w http.ResponseWriter, _ *http.Request, userID, _ string) {
	user, err := s.auth.Profile(userID)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{
		"id": user.ID, "email": user.Email, "address": user.Address, "created_at": user.CreatedAt,
	})
}

// exportAccount returns all personal data held about the account, as a download.
func (s *Server) exportAccount(w http.ResponseWriter, _ *http.Request, userID, _ string) {
	data, err := s.store.ExportOf(userID)
	if err != nil {
		s.fail(w, err)
		return
	}
	w.Header().Set("Content-Disposition", `attachment; filename="silicium-account-data.json"`)
	httpx.WriteJSON(w, http.StatusOK, data)
}

// deleteAccount erases the account. An e-mail account confirms with its password; a wallet
// account confirms with a fresh signature of a delete challenge.
func (s *Server) deleteAccount(w http.ResponseWriter, r *http.Request, userID, _ string) {
	var in struct {
		Password  string `json:"password"`
		Nonce     string `json:"nonce"`
		Signature string `json:"signature"`
	}
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	if !s.loginEmail.Allow("delete:"+userID, time.Now()) {
		httpx.WriteError(w, http.StatusTooManyRequests, "rate_limited", "too many attempts")
		return
	}
	var err error
	if in.Nonce != "" {
		signature, decodeErr := base64.StdEncoding.DecodeString(in.Signature)
		if decodeErr != nil {
			httpx.WriteError(w, http.StatusBadRequest, "invalid_input", "signature is not valid base64")
			return
		}
		err = s.auth.DeleteAccountWithWallet(userID, in.Nonce, signature)
	} else {
		err = s.auth.DeleteAccount(userID, in.Password)
	}
	if err != nil {
		s.fail(w, err)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

// setCredentials attaches an e-mail and a password to the account, or changes them. Changing an
// existing password needs the current one; the attempts share the sign-in rate limit.
func (s *Server) setCredentials(w http.ResponseWriter, r *http.Request, userID, _ string) {
	var in struct {
		Email           string `json:"email"`
		CurrentPassword string `json:"current_password"`
		NewPassword     string `json:"new_password"`
	}
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	if !s.loginEmail.Allow("credentials:"+userID, time.Now()) {
		httpx.WriteError(w, http.StatusTooManyRequests, "rate_limited", "too many attempts")
		return
	}
	if err := s.auth.SetCredentials(userID, in.Email, in.CurrentPassword, in.NewPassword); err != nil {
		s.fail(w, err)
		return
	}
	s.profile(w, r, userID, "")
}
