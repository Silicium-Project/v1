package httpx

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

var t0 = time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)

func TestLimiterAllowsABurstThenRefills(t *testing.T) {
	l := NewLimiter(1, 3, 10)
	for i := 0; i < 3; i++ {
		if !l.Allow("a", t0) {
			t.Fatalf("burst %d refused", i)
		}
	}
	if l.Allow("a", t0) {
		t.Fatal("over the burst")
	}
	if !l.Allow("a", t0.Add(1100*time.Millisecond)) {
		t.Fatal("no refill after a second")
	}
	if !l.Allow("b", t0) {
		t.Fatal("keys must be independent")
	}
}

func TestLimiterMemoryIsBounded(t *testing.T) {
	l := NewLimiter(0.001, 1, 50)
	for i := 0; i < 1000; i++ {
		l.Allow(string(rune('a'+i%26))+strings.Repeat("k", i), t0.Add(time.Duration(i)*time.Millisecond))
	}
	if len(l.buckets) > 50 {
		t.Fatalf("%d keys kept", len(l.buckets))
	}
}

func TestRateLimitAnswers429WithRetryAfter(t *testing.T) {
	handler := RateLimit(NewLimiter(0.001, 1, 10))(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {}))
	codes := []int{}
	for i := 0; i < 2; i++ {
		recorder := httptest.NewRecorder()
		request := httptest.NewRequest("GET", "/", nil)
		request.RemoteAddr = "203.0.113.5:4000"
		handler.ServeHTTP(recorder, request)
		codes = append(codes, recorder.Code)
		if recorder.Code == 429 && recorder.Header().Get("Retry-After") == "" {
			t.Fatal("no Retry-After")
		}
	}
	if codes[0] != 200 || codes[1] != 429 {
		t.Fatalf("%v", codes)
	}
}

func TestForwardedHeadersAreIgnoredForTheClientKey(t *testing.T) {
	request := httptest.NewRequest("GET", "/", nil)
	request.RemoteAddr = "203.0.113.5:4000"
	request.Header.Set("X-Forwarded-For", "1.2.3.4")
	if got := ClientKey(request); got != "203.0.113.5" {
		t.Fatal(got)
	}
}

func TestRecoverHidesPanicDetails(t *testing.T) {
	handler := Recover(nopLogger())(http.HandlerFunc(func(http.ResponseWriter, *http.Request) { panic("secret detail") }))
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, httptest.NewRequest("GET", "/", nil))
	if recorder.Code != 500 || strings.Contains(recorder.Body.String(), "secret") {
		t.Fatalf("%d %s", recorder.Code, recorder.Body.String())
	}
}
