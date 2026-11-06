package server

import (
	"bytes"
	"crypto/ed25519"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"io"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	"sil/api/internal/auth"
	"sil/api/internal/devices"
	"sil/api/internal/jobs"
	"sil/api/internal/nodeclient"
	"sil/api/internal/store"
)

var jobHex = strings.Repeat("cd", 32)

// fakeNode is a node control API: it knows the job once an event was submitted.
type fakeNode struct {
	server    *httptest.Server
	submitted atomic.Bool
	down      atomic.Bool
	state     atomic.Value // string
}

func newFakeNode(t *testing.T) *fakeNode {
	t.Helper()
	n := &fakeNode{}
	n.state.Store("Active")
	n.server = httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if n.down.Load() {
			w.WriteHeader(http.StatusServiceUnavailable)
			return
		}
		if r.Header.Get("Authorization") != "Bearer node-secret" {
			w.WriteHeader(http.StatusUnauthorized)
			return
		}
		switch {
		case r.Method == http.MethodPut && r.URL.Path == "/v1/content":
			raw, _ := io.ReadAll(r.Body)
			w.WriteHeader(http.StatusCreated)
			_ = json.NewEncoder(w).Encode(map[string]any{"content_id": strings.Repeat("11", 32), "size": len(raw)})
		case r.Method == http.MethodPost && r.URL.Path == "/v1/events":
			n.submitted.Store(true)
			w.WriteHeader(http.StatusAccepted)
			_, _ = w.Write([]byte(`{"accepted":1,"pending":false}`))
		case r.Method == http.MethodGet && r.URL.Path == "/v1/jobs/"+jobHex && n.submitted.Load():
			_, _ = w.Write([]byte(`{"job":"` + jobHex + `","status":"` + n.state.Load().(string) + `","tasks":[{"status":"Open"}]}`))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	t.Cleanup(n.server.Close)
	return n
}

type env struct {
	api  *httptest.Server
	node *fakeNode
}

func newEnv(t *testing.T, allowRegistration bool) *env {
	t.Helper()
	return newEnvWith(t, allowRegistration, "")
}

// newEnvWeb serves a built web app from dir, as the production server does.
func newEnvWeb(t *testing.T, dir string) *env {
	t.Helper()
	return newEnvWith(t, true, dir)
}

func newEnvWith(t *testing.T, allowRegistration bool, webDir string) *env {
	t.Helper()
	data, _ := store.Open("")
	node := newFakeNode(t)
	client := nodeclient.New(node.server.URL, "node-secret", 2*time.Second)
	s := New(
		data,
		auth.New(data, auth.Config{Iterations: 1000, SessionTTL: time.Hour}, nil),
		devices.New(data, nil),
		jobs.New(data, client, nil),
		Options{AllowRegistration: allowRegistration, WebDir: webDir, Log: slog.New(slog.NewTextHandler(io.Discard, nil))},
	)
	// Raise the limits: rate limiting has its own test.
	s.general = limiterFor(1000, 1000)
	s.sensitive = limiterFor(1000, 1000)
	api := httptest.NewServer(s.Handler())
	t.Cleanup(api.Close)
	return &env{api: api, node: node}
}

func (e *env) call(t *testing.T, method, path, token string, body any) (int, map[string]any) {
	t.Helper()
	var reader io.Reader
	switch b := body.(type) {
	case nil:
	case []byte:
		reader = bytes.NewReader(b)
	default:
		raw, _ := json.Marshal(b)
		reader = bytes.NewReader(raw)
	}
	request, _ := http.NewRequest(method, e.api.URL+path, reader)
	if token != "" {
		request.Header.Set("Authorization", "Bearer "+token)
	}
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	var out map[string]any
	_ = json.NewDecoder(response.Body).Decode(&out)
	return response.StatusCode, out
}

func (e *env) login(t *testing.T, email string) string {
	t.Helper()
	creds := map[string]string{"email": email, "password": "correct horse battery"}
	if status, _ := e.call(t, "POST", "/v1/auth/register", "", creds); status != 201 {
		t.Fatalf("register: %d", status)
	}
	status, out := e.call(t, "POST", "/v1/auth/login", "", creds)
	if status != 200 {
		t.Fatalf("login: %d", status)
	}
	return out["token"].(string)
}

func TestWholeUserJourney(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "alice@example.com")

	// Link a machine.
	_, out := e.call(t, "POST", "/v1/devices/link-codes", token, nil)
	code := out["code"].(string)
	key := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{7}, 32))
	pub := key.Public().(ed25519.PublicKey)
	sig := ed25519.Sign(key, devices.Frame(devices.Message(code, pub)))
	status, device := e.call(t, "POST", "/v1/devices/link", "", map[string]string{
		"code": code, "public_key": hex.EncodeToString(pub), "signature": hex.EncodeToString(sig), "label": "pc",
	})
	if status != 201 {
		t.Fatalf("link: %d %v", status, device)
	}
	if status, list := e.call(t, "GET", "/v1/devices", token, nil); status != 200 || len(list["devices"].([]any)) != 1 {
		t.Fatalf("devices: %d %v", status, list)
	}

	// Upload an input and publish a signed event.
	if status, out := e.call(t, "PUT", "/v1/content", token, []byte("input")); status != 201 || out["size"].(float64) != 5 {
		t.Fatalf("content: %d %v", status, out)
	}
	publish := map[string]string{"job_id": jobHex, "event": base64.StdEncoding.EncodeToString([]byte("signed")), "label": "render"}
	if status, out := e.call(t, "POST", "/v1/jobs", token, publish); status != 201 {
		t.Fatalf("publish: %d %v", status, out)
	}
	_, view := e.call(t, "GET", "/v1/jobs/"+jobHex, token, nil)
	if got := view["status"].(map[string]any)["state"]; got != "running" {
		t.Fatalf("state %v", got)
	}

	// The node goes away: the answer is "unknown", never a stale or invented state.
	e.node.down.Store(true)
	_, view = e.call(t, "GET", "/v1/jobs/"+jobHex, token, nil)
	if got := view["status"].(map[string]any)["state"]; got != "unknown" {
		t.Fatalf("with the node down: %v", got)
	}
	e.node.down.Store(false)
	e.node.state.Store("Finalized")
	_, view = e.call(t, "GET", "/v1/jobs/"+jobHex, token, nil)
	if got := view["status"].(map[string]any)["state"]; got != "finalized" {
		t.Fatalf("state %v", got)
	}

	if status, _ := e.call(t, "POST", "/v1/auth/logout", token, nil); status != 204 {
		t.Fatalf("logout: %d", status)
	}
	if status, _ := e.call(t, "GET", "/v1/jobs", token, nil); status != 401 {
		t.Fatalf("after logout: %d", status)
	}
}
