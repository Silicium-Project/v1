package httpx

import (
	"net/http"
	"strings"
)

// CORS allows only the listed origins (the desktop shell's webview, for example). Nothing
// else is allowed to read answers; an empty list means no CORS headers at all. Credentials
// are never allowed: the bearer token is sent explicitly, not as a cookie.
func CORS(origins []string) Middleware {
	allowed := make(map[string]bool, len(origins))
	for _, origin := range origins {
		allowed[strings.TrimRight(origin, "/")] = true
	}
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			origin := r.Header.Get("Origin")
			if origin == "" || len(allowed) == 0 {
				next.ServeHTTP(w, r)
				return
			}
			h := w.Header()
			h.Add("Vary", "Origin")
			if !allowed[origin] {
				next.ServeHTTP(w, r)
				return
			}
			h.Set("Access-Control-Allow-Origin", origin)
			h.Set("Access-Control-Expose-Headers", "X-Request-Id")
			if r.Method == http.MethodOptions {
				h.Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE")
				h.Set("Access-Control-Allow-Headers", "Authorization, Content-Type")
				h.Set("Access-Control-Max-Age", "600")
				w.WriteHeader(http.StatusNoContent)
				return
			}
			next.ServeHTTP(w, r)
		})
	}
}
