package server

import (
	"encoding/base64"
	"net/http"

	"sil/api/internal/httpx"
	"sil/api/internal/wallet"
)

type walletChallengeRequest struct {
	Address string `json:"address"`
}

// walletChallenge gives the message a wallet must sign to log in. No secret is involved:
// the nonce is random, single-use and short-lived.
func (s *Server) walletChallenge(w http.ResponseWriter, r *http.Request) {
	var in walletChallengeRequest
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	nonce, message, expires, err := s.auth.WalletChallenge(in.Address, wallet.PurposeLogin)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, map[string]any{
		"nonce": nonce, "message": message, "expires_at": expires,
	})
}

type walletLoginRequest struct {
	Address   string `json:"address"`
	Nonce     string `json:"nonce"`
	Signature string `json:"signature"` // base64 of the 64-byte Ed25519 signature
}

func (s *Server) walletLogin(w http.ResponseWriter, r *http.Request) {
	var in walletLoginRequest
	if err := httpx.ReadJSON(w, r, &in, 4096); err != nil {
		badBody(w, err)
		return
	}
	signature, err := base64.StdEncoding.DecodeString(in.Signature)
	if err != nil {
		httpx.WriteError(w, http.StatusBadRequest, "invalid_input", "signature is not valid base64")
		return
	}
	token, expires, err := s.auth.LoginWallet(in.Address, in.Nonce, signature)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusOK, map[string]any{"token": token, "expires_at": expires})
}

// deleteChallenge gives the message to sign before a wallet account can be deleted.
func (s *Server) deleteChallenge(w http.ResponseWriter, _ *http.Request, userID, _ string) {
	nonce, message, expires, err := s.auth.DeleteChallenge(userID)
	if err != nil {
		s.fail(w, err)
		return
	}
	httpx.WriteJSON(w, http.StatusCreated, map[string]any{
		"nonce": nonce, "message": message, "expires_at": expires,
	})
}
