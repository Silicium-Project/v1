package httpx

import (
	"net/http"
	"net/http/httptest"
	"testing"
)

func corsHandler(origins ...string) http.Handler {
	return CORS(origins)(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))
}

func TestListedOriginIsAllowedWithoutCredentials(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/v1/jobs", nil)
	request.Header.Set("Origin", "http://tauri.localhost")
	corsHandler("http://tauri.localhost/").ServeHTTP(recorder, request)
	if got := recorder.Header().Get("Access-Control-Allow-Origin"); got != "http://tauri.localhost" {
		t.Fatalf("allow-origin = %q", got)
	}
	if recorder.Header().Get("Access-Control-Allow-Credentials") != "" {
		t.Fatal("credentials must never be allowed")
	}
}

func TestUnlistedOriginGetsNoCorsHeader(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/v1/jobs", nil)
	request.Header.Set("Origin", "https://evil.example")
	corsHandler("http://tauri.localhost").ServeHTTP(recorder, request)
	if recorder.Header().Get("Access-Control-Allow-Origin") != "" {
		t.Fatal("unlisted origin was allowed")
	}
}

func TestPreflightAnswersWithoutRunningTheHandler(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodOptions, "/v1/jobs", nil)
	request.Header.Set("Origin", "tauri://localhost")
	ran := false
	handler := CORS([]string{"tauri://localhost"})(http.HandlerFunc(func(http.ResponseWriter, *http.Request) {
		ran = true
	}))
	handler.ServeHTTP(recorder, request)
	if recorder.Code != http.StatusNoContent || ran {
		t.Fatalf("status %d, handler ran: %v", recorder.Code, ran)
	}
	if recorder.Header().Get("Access-Control-Allow-Headers") != "Authorization, Content-Type" {
		t.Fatalf("headers: %v", recorder.Header())
	}
}

func TestNoOriginListMeansNoCors(t *testing.T) {
	recorder := httptest.NewRecorder()
	request := httptest.NewRequest(http.MethodGet, "/", nil)
	request.Header.Set("Origin", "http://tauri.localhost")
	corsHandler().ServeHTTP(recorder, request)
	if recorder.Header().Get("Access-Control-Allow-Origin") != "" {
		t.Fatal("CORS enabled without configuration")
	}
}
