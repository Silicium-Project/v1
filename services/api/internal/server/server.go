// Package server wires services to HTTP routes.
package server

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"strings"
	"time"

	"sil/api/internal/auth"
	"sil/api/internal/devices"
	"sil/api/internal/httpx"
	"sil/api/internal/jobs"
	"sil/api/internal/nodeclient"
	"sil/api/internal/store"
	"sil/api/internal/wallet"
)

// Body limits per route family.
const (
	maxJSONBody    = 128 * 1024 // job events travel as base64 inside JSON
	maxContentBody = jobs.MaxContentBytes
)

// Options configures the server.
type Options struct {
	AllowRegistration bool
	// WebDir is the built web app served at "/" (empty: API only).
	WebDir string
	// CORSOrigins lists the origins allowed to call the API (empty: none).
	CORSOrigins []string
	// Committee is given to new jobs (coordinator keys, hex).
	Committee []string
	Log       *slog.Logger
}

// Server holds the services and the limiters.
type Server struct {
	auth    *auth.Service
	store   *store.Store
	devices *devices.Service
	jobs    *jobs.Service
	log     *slog.Logger
	opts    Options

	general    *httpx.Limiter // every request, per client address
	sensitive  *httpx.Limiter // register, login, device link: per client address
	loginEmail *httpx.Limiter // login attempts per e-mail, slow refill
}

// New builds the server.
func New(data *store.Store, a *auth.Service, d *devices.Service, j *jobs.Service, opts Options) *Server {
	if opts.Log == nil {
		opts.Log = slog.Default()
	}
	return &Server{
		store: data, auth: a, devices: d, jobs: j, log: opts.Log, opts: opts,
		general:    httpx.NewLimiter(30, 90, 10_000),
		sensitive:  httpx.NewLimiter(1, 10, 10_000),
		loginEmail: httpx.NewLimiter(1.0/180, 5, 10_000),
	}
}

// Handler returns the full HTTP handler with its middleware.
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /health", func(w http.ResponseWriter, _ *http.Request) {
		httpx.WriteJSON(w, http.StatusOK, map[string]string{"status": "ok"})
	})
	mux.Handle("POST /v1/auth/register", s.limited(s.register))
	mux.Handle("POST /v1/auth/login", s.limited(s.login))
	mux.HandleFunc("POST /v1/auth/logout", s.authed(s.logout))
	mux.Handle("POST /v1/auth/wallet/challenge", s.limited(s.walletChallenge))
	mux.Handle("POST /v1/auth/wallet/login", s.limited(s.walletLogin))
	mux.HandleFunc("POST /v1/account/delete-challenge", s.authed(s.deleteChallenge))
	mux.HandleFunc("POST /v1/devices/link-codes", s.authed(s.newLinkCode))
	mux.Handle("POST /v1/devices/link", s.limited(s.linkDevice))
	mux.HandleFunc("GET /v1/devices", s.authed(s.listDevices))
	mux.HandleFunc("DELETE /v1/devices/{id}", s.authed(s.revokeDevice))
	mux.HandleFunc("GET /v1/network/committee", s.authed(s.committee))
	mux.HandleFunc("GET /v1/account", s.authed(s.profile))
	mux.HandleFunc("GET /v1/account/export", s.authed(s.exportAccount))
	mux.HandleFunc("DELETE /v1/account", s.authed(s.deleteAccount))
	mux.HandleFunc("PUT /v1/content", s.authed(s.uploadContent))
	mux.HandleFunc("POST /v1/jobs", s.authed(s.publishJob))
	mux.HandleFunc("GET /v1/jobs", s.authed(s.listJobs))
	mux.HandleFunc("GET /v1/jobs/{id}", s.authed(s.getJob))
	mux.Handle("/v1/", http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		httpx.WriteError(w, http.StatusNotFound, "not_found", "unknown route")
	}))
	if s.opts.WebDir != "" {
		mux.Handle("/", spaHandler(s.opts.WebDir))
	}
	return httpx.Chain(mux,
		httpx.Recover(s.log),
		httpx.AccessLog(s.log),
		httpx.CORS(s.opts.CORSOrigins),
		httpx.SecurityHeaders,
		httpx.RateLimit(s.general),
	)
}

// limited applies the stricter per-address limit to unauthenticated, abuse-prone routes.
func (s *Server) limited(next http.HandlerFunc) http.Handler {
	return httpx.RateLimit(s.sensitive)(next)
}

type authedHandler func(w http.ResponseWriter, r *http.Request, userID, token string)

// authed requires a valid session token in "Authorization: Bearer".
func (s *Server) authed(next authedHandler) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		token, ok := strings.CutPrefix(r.Header.Get("Authorization"), "Bearer ")
		if !ok {
			httpx.WriteError(w, http.StatusUnauthorized, "unauthenticated", "missing bearer token")
			return
		}
		userID, err := s.auth.Authenticate(token)
		if err != nil {
			httpx.WriteError(w, http.StatusUnauthorized, "unauthenticated", "invalid or expired token")
			return
		}
		next(w, r, userID, token)
	}
}

// fail maps a domain error to an HTTP answer. Unknown errors become a generic 500 and are
// logged; their text is never sent to the client.
func (s *Server) fail(w http.ResponseWriter, err error) {
	type mapping struct {
		target error
		status int
		code   string
	}
	for _, m := range []mapping{
		{auth.ErrInvalidInput, 400, "invalid_input"},
		{devices.ErrInvalidInput, 400, "invalid_input"},
		{jobs.ErrInvalidInput, 400, "invalid_input"},
		{auth.ErrEmailTaken, 409, "email_taken"},
		{auth.ErrInvalidCredentials, 401, "invalid_credentials"},
		{devices.ErrInvalidProof, 400, "invalid_proof"},
		{devices.ErrBadCode, 400, "bad_link_code"},
		{devices.ErrKeyInUse, 409, "key_in_use"},
		{devices.ErrTooMany, 409, "too_many_devices"},
		{jobs.ErrJobTaken, 409, "job_taken"},
		{jobs.ErrTooMany, 409, "too_many_jobs"},
		{jobs.ErrNotRecorded, 422, "not_a_job"},
		{jobs.ErrRejected, 422, "node_rejected"},
		{jobs.ErrUnavailable, 503, "node_unavailable"},
		{jobs.ErrNotFound, 404, "not_found"},
		{wallet.ErrFull, 503, "busy"},
		{store.ErrNotFound, 404, "not_found"},
		{nodeclient.ErrNotFound, 404, "not_found"},
	} {
		if errors.Is(err, m.target) {
			httpx.WriteError(w, m.status, m.code, m.target.Error())
			return
		}
	}
	s.log.Error("unexpected error", "error", err)
	httpx.WriteError(w, http.StatusInternalServerError, "internal", "internal error")
}

// badBody answers a body that could not be read or decoded.
func badBody(w http.ResponseWriter, err error) {
	if httpx.IsTooLarge(err) {
		httpx.WriteError(w, http.StatusRequestEntityTooLarge, "too_large", "request body too large")
		return
	}
	httpx.WriteError(w, http.StatusBadRequest, "invalid_input", "malformed request body")
}

func requestContext(r *http.Request) (context.Context, context.CancelFunc) {
	return context.WithTimeout(r.Context(), 30*time.Second)
}
