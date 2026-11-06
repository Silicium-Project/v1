package httpx

import (
	"crypto/rand"
	"encoding/hex"
	"log/slog"
	"net"
	"net/http"
	"strings"
	"time"
)

// Middleware wraps a handler.
type Middleware func(http.Handler) http.Handler

// Chain applies middlewares so that the first one is the outermost.
func Chain(h http.Handler, middlewares ...Middleware) http.Handler {
	for i := len(middlewares) - 1; i >= 0; i-- {
		h = middlewares[i](h)
	}
	return h
}

// Recover turns a panic into a 500 without leaking details.
func Recover(log *slog.Logger) Middleware {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			defer func() {
				if recovered := recover(); recovered != nil {
					log.Error("panic in handler", "path", r.URL.Path, "panic", recovered)
					WriteError(w, http.StatusInternalServerError, "internal", "internal error")
				}
			}()
			next.ServeHTTP(w, r)
		})
	}
}

// SecurityHeaders sets the headers of the API and of the single-page app. The CSP forbids
// inline scripts and styles, third-party origins and framing; API answers get the strictest
// policy (they never render).
func SecurityHeaders(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		h := w.Header()
		h.Set("X-Content-Type-Options", "nosniff")
		h.Set("Referrer-Policy", "no-referrer")
		h.Set("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=()")
		if strings.HasPrefix(r.URL.Path, "/v1/") || r.URL.Path == "/health" {
			h.Set("Cache-Control", "no-store")
			h.Set("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'")
		} else {
			h.Set("Content-Security-Policy", "default-src 'self'; img-src 'self' data:; style-src 'self'; "+
				"script-src 'self'; connect-src 'self'; font-src 'self'; object-src 'none'; "+
				"base-uri 'none'; form-action 'self'; frame-ancestors 'none'")
		}
		next.ServeHTTP(w, r)
	})
}

type statusWriter struct {
	http.ResponseWriter
	status int
}

func (s *statusWriter) WriteHeader(status int) {
	s.status = status
	s.ResponseWriter.WriteHeader(status)
}

// AccessLog logs method, path, status and duration with a request id. It never logs
// headers, query strings or bodies (they may carry tokens).
func AccessLog(log *slog.Logger) Middleware {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			var raw [8]byte
			_, _ = rand.Read(raw[:])
			id := hex.EncodeToString(raw[:])
			w.Header().Set("X-Request-Id", id)
			writer := &statusWriter{ResponseWriter: w, status: http.StatusOK}
			start := time.Now()
			next.ServeHTTP(writer, r)
			log.Info("request", "id", id, "method", r.Method, "path", r.URL.Path,
				"status", writer.status, "ms", time.Since(start).Milliseconds())
		})
	}
}

// ClientKey identifies the caller for rate limiting: the TCP peer address. Forwarded
// headers are ignored on purpose (any client can forge them); deploy behind a proxy that
// rewrites RemoteAddr if per-user addresses are needed.
func ClientKey(r *http.Request) string {
	host, _, err := net.SplitHostPort(r.RemoteAddr)
	if err != nil {
		return r.RemoteAddr
	}
	return host
}

// RateLimit rejects callers above the limiter's rate with HTTP 429.
func RateLimit(l *Limiter) Middleware {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if !l.Allow(ClientKey(r), time.Now()) {
				w.Header().Set("Retry-After", "1")
				WriteError(w, http.StatusTooManyRequests, "rate_limited", "too many requests")
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}
