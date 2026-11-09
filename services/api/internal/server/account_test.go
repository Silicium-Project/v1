package server

import (
	"encoding/base64"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestExportHoldsNoSecretsAndDeleteNeedsThePassword(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "carol@example.com")
	body := map[string]string{"job_id": jobHex, "event": base64.StdEncoding.EncodeToString([]byte("e")), "label": "mine"}
	if status, _ := e.call(t, "POST", "/v1/jobs", token, body); status != 201 {
		t.Fatalf("publish: %d", status)
	}

	status, export := e.call(t, "GET", "/v1/account/export", token, nil)
	if status != 200 || export["email"] != "carol@example.com" {
		t.Fatalf("export: %d %v", status, export)
	}
	if _, leaked := export["password"]; leaked {
		t.Fatal("export contains a password field")
	}
	if len(export["jobs"].([]any)) != 1 {
		t.Fatalf("export jobs: %v", export["jobs"])
	}

	if status, _ := e.call(t, "DELETE", "/v1/account", token, map[string]string{"password": "wrong password here"}); status != 401 {
		t.Fatalf("delete with wrong password: %d", status)
	}
	if status, _ := e.call(t, "DELETE", "/v1/account", token, map[string]string{"password": "correct horse battery"}); status != 204 {
		t.Fatalf("delete: %d", status)
	}
	if status, _ := e.call(t, "GET", "/v1/jobs", token, nil); status != 401 {
		t.Fatalf("session survived the deletion: %d", status)
	}
	if status, _ := e.call(t, "POST", "/v1/auth/login", "", map[string]string{"email": "carol@example.com", "password": "correct horse battery"}); status != 401 {
		t.Fatalf("login after deletion: %d", status)
	}
}

func TestSinglePageAppServesFilesAndFallsBackToIndex(t *testing.T) {
	dir := t.TempDir()
	must(t, os.WriteFile(filepath.Join(dir, "index.html"), []byte("<!doctype html><title>app</title>"), 0o600))
	must(t, os.MkdirAll(filepath.Join(dir, "assets"), 0o700))
	must(t, os.WriteFile(filepath.Join(dir, "assets", "app.js"), []byte("console.log(1)"), 0o600))

	e := newEnvWeb(t, dir)
	get := func(path string) (int, string, string) {
		response, err := http.Get(e.api.URL + path)
		if err != nil {
			t.Fatal(err)
		}
		defer response.Body.Close()
		var b strings.Builder
		buf := make([]byte, 512)
		for {
			n, readErr := response.Body.Read(buf)
			b.Write(buf[:n])
			if readErr != nil {
				break
			}
		}
		return response.StatusCode, b.String(), response.Header.Get("Cache-Control")
	}

	status, body, cache := get("/assets/app.js")
	if status != 200 || !strings.Contains(body, "console.log") || !strings.Contains(cache, "immutable") {
		t.Fatalf("asset: %d %q %q", status, body, cache)
	}
	status, body, _ = get("/machines/deep/link")
	if status != 200 || !strings.Contains(body, "<title>app</title>") {
		t.Fatalf("client route: %d %q", status, body)
	}
	if status, _, _ := get("/v1/unknown"); status != 404 {
		t.Fatalf("unknown API route: %d", status)
	}
}

func must(t *testing.T, err error) {
	t.Helper()
	if err != nil {
		t.Fatal(err)
	}
}

func TestCredentialsChangeNeedsTheCurrentPassword(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "dave@example.com")
	_ = e.login(t, "erin@example.com") // the e-mail that must stay taken
	const password = "correct horse battery"

	change := func(token string, body map[string]string) (int, map[string]any) {
		return e.call(t, "PUT", "/v1/account/credentials", token, body)
	}
	if status, out := change(token, map[string]string{"email": "dave@example.com", "current_password": "wrong password here"}); status != 403 {
		t.Fatalf("wrong current password: %d %v", status, out)
	}
	if status, _ := change(token, map[string]string{"email": "erin@example.com", "current_password": password}); status != 409 {
		t.Fatalf("taken e-mail: %d", status)
	}
	if status, out := change(token, map[string]string{"email": "dave.new@example.com", "current_password": password}); status != 200 || out["email"] != "dave.new@example.com" {
		t.Fatalf("change e-mail: %d %v", status, out)
	}
	if status, _ := e.call(t, "POST", "/v1/auth/login", "", map[string]string{"email": "dave.new@example.com", "password": password}); status != 200 {
		t.Fatalf("login with the new e-mail: %d", status)
	}
}
