package server

import (
	"bytes"
	"encoding/base64"
	"net/http"
	"strings"
	"testing"

	"sil/api/internal/httpx"
)

func limiterFor(perSecond float64, burst int) *httpx.Limiter {
	return httpx.NewLimiter(perSecond, burst, 1000)
}

func TestRoutesThatNeedASessionRefuseAnonymousAndGarbageTokens(t *testing.T) {
	e := newEnv(t, true)
	routes := [][2]string{
		{"POST", "/v1/auth/logout"}, {"POST", "/v1/devices/link-codes"}, {"GET", "/v1/devices"},
		{"DELETE", "/v1/devices/x"}, {"DELETE", "/v1/devices/x/purge"}, {"PUT", "/v1/content"}, {"POST", "/v1/jobs"},
		{"GET", "/v1/jobs"}, {"GET", "/v1/jobs/" + jobHex},
	}
	for _, token := range []string{"", "nope", "sils_" + strings.Repeat("A", 43)} {
		for _, route := range routes {
			if status, _ := e.call(t, route[0], route[1], token, nil); status != 401 {
				t.Errorf("%s %s with %q: %d", route[0], route[1], token, status)
			}
		}
	}
}

func TestAnotherUserCannotSeeMyJob(t *testing.T) {
	e := newEnv(t, true)
	alice := e.login(t, "alice@example.com")
	mallory := e.login(t, "mallory@example.com")
	body := map[string]string{"job_id": jobHex, "event": base64.StdEncoding.EncodeToString([]byte("e"))}
	if status, _ := e.call(t, "POST", "/v1/jobs", alice, body); status != 201 {
		t.Fatalf("publish: %d", status)
	}
	if status, _ := e.call(t, "GET", "/v1/jobs/"+jobHex, mallory, nil); status != 404 {
		t.Fatalf("read by another user: %d", status)
	}
	if status, _ := e.call(t, "POST", "/v1/jobs", mallory, body); status != 409 {
		t.Fatalf("claim by another user: %d", status)
	}
	if _, list := e.call(t, "GET", "/v1/jobs", mallory, nil); len(list["jobs"].([]any)) != 0 {
		t.Fatalf("listed: %v", list)
	}
}

func TestPublishWhileTheNodeIsDownIsAnHonest503AndRecordsNothing(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "alice@example.com")
	e.node.down.Store(true)
	body := map[string]string{"job_id": jobHex, "event": base64.StdEncoding.EncodeToString([]byte("e"))}
	status, out := e.call(t, "POST", "/v1/jobs", token, body)
	if status != 503 || out["code"] != "node_unavailable" {
		t.Fatalf("%d %v", status, out)
	}
	if _, list := e.call(t, "GET", "/v1/jobs", token, nil); len(list["jobs"].([]any)) != 0 {
		t.Fatalf("recorded: %v", list)
	}
}

func TestRegistrationCanBeClosed(t *testing.T) {
	e := newEnv(t, false)
	status, out := e.call(t, "POST", "/v1/auth/register", "", map[string]string{"email": "a@b.co", "password": "correct horse battery"})
	if status != 403 || out["code"] != "registration_closed" {
		t.Fatalf("%d %v", status, out)
	}
}

func TestLoginErrorsDoNotRevealWhetherTheAccountExists(t *testing.T) {
	e := newEnv(t, true)
	e.login(t, "alice@example.com")
	s1, o1 := e.call(t, "POST", "/v1/auth/login", "", map[string]string{"email": "alice@example.com", "password": "wrong wrong wrong"})
	s2, o2 := e.call(t, "POST", "/v1/auth/login", "", map[string]string{"email": "nobody@example.com", "password": "wrong wrong wrong"})
	if s1 != 401 || s2 != 401 || o1["message"] != o2["message"] {
		t.Fatalf("%d %v / %d %v", s1, o1, s2, o2)
	}
}

func TestPasswordGuessingOnOneAccountIsThrottled(t *testing.T) {
	e := newEnv(t, true)
	e.login(t, "alice@example.com")
	bad := map[string]string{"email": "alice@example.com", "password": "wrong wrong wrong"}
	last := 0
	for i := 0; i < 8; i++ {
		last, _ = e.call(t, "POST", "/v1/auth/login", "", bad)
	}
	if last != 429 {
		t.Fatalf("still answering after many guesses: %d", last)
	}
	// Even the right password is refused while throttled.
	good := map[string]string{"email": "alice@example.com", "password": "correct horse battery"}
	if status, _ := e.call(t, "POST", "/v1/auth/login", "", good); status != 429 {
		t.Fatalf("right password during throttle: %d", status)
	}
}

func TestOversizedAndMalformedBodiesAreRefused(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "alice@example.com")
	huge := append(append([]byte(`{"event":"`), bytes.Repeat([]byte("x"), 200*1024)...), []byte(`"}`)...)
	if status, _ := e.call(t, "POST", "/v1/jobs", token, huge); status != http.StatusRequestEntityTooLarge {
		t.Errorf("huge JSON: %d", status)
	}
	for name, body := range map[string][]byte{
		"not json":      []byte("{"),
		"unknown field": []byte(`{"email":"a@b.co","password":"correct horse battery","admin":true}`),
		"trailing data": []byte(`{"email":"a@b.co","password":"correct horse battery"}{}`),
	} {
		if status, _ := e.call(t, "POST", "/v1/auth/register", "", body); status != 400 {
			t.Errorf("%s: %d", name, status)
		}
	}
	bad := map[string]string{"job_id": jobHex, "event": "***not base64***"}
	if status, _ := e.call(t, "POST", "/v1/jobs", token, bad); status != 400 {
		t.Errorf("bad base64: %d", status)
	}
}

func TestLinkingWithAForgedProofFails(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "alice@example.com")
	_, out := e.call(t, "POST", "/v1/devices/link-codes", token, nil)
	forged := map[string]string{
		"code": out["code"].(string), "public_key": strings.Repeat("ab", 32), "signature": strings.Repeat("00", 64),
	}
	status, answer := e.call(t, "POST", "/v1/devices/link", "", forged)
	if status != 400 || answer["code"] != "invalid_proof" {
		t.Fatalf("%d %v", status, answer)
	}
	// The failed attempt must not burn the code: the legitimate machine can still link.
	if _, list := e.call(t, "GET", "/v1/devices", token, nil); len(list["devices"].([]any)) != 0 {
		t.Fatalf("device registered: %v", list)
	}
}

func TestResponsesCarrySecurityHeadersAndNoStore(t *testing.T) {
	e := newEnv(t, true)
	response, err := http.Get(e.api.URL + "/health")
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	h := response.Header
	if h.Get("X-Content-Type-Options") != "nosniff" || h.Get("Cache-Control") != "no-store" || h.Get("X-Request-Id") == "" {
		t.Fatalf("headers: %v", h)
	}
}
