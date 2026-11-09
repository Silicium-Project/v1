package config

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func env(values map[string]string) func(string) string {
	return func(key string) string { return values[key] }
}

func tokenFile(t *testing.T, content string) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "node.token")
	if err := os.WriteFile(path, []byte(content), 0o600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestDefaultsAndTokenSelection(t *testing.T) {
	file := tokenFile(t, "# comment\nread:onlyread-secret\nread,submit:the-real-secret\n")
	cfg, err := Load(env(map[string]string{
		"SIL_API_NODE_URL": "http://127.0.0.1:8750", "SIL_API_NODE_TOKEN_FILE": file,
	}))
	if err != nil {
		t.Fatal(err)
	}
	if cfg.NodeToken != "the-real-secret" || cfg.Addr != "127.0.0.1:8080" || !cfg.AllowRegistration {
		t.Fatalf("%+v", cfg)
	}
}

func TestEveryProblemIsReportedAtOnce(t *testing.T) {
	_, err := Load(env(map[string]string{
		"SIL_API_NODE_URL": "ftp://x", "SIL_API_ALLOW_REGISTRATION": "maybe", "SIL_API_SESSION_HOURS": "0",
	}))
	if err == nil {
		t.Fatal("expected errors")
	}
	for _, want := range []string{"NODE_URL", "ALLOW_REGISTRATION", "SESSION_HOURS", "NODE_TOKEN_FILE"} {
		if !strings.Contains(err.Error(), want) {
			t.Errorf("missing %s in %v", want, err)
		}
	}
}

func TestTokenFileWithoutBothScopesIsRefused(t *testing.T) {
	file := tokenFile(t, "read:abc\nsubmit:def\n")
	_, err := Load(env(map[string]string{"SIL_API_NODE_URL": "http://h:1", "SIL_API_NODE_TOKEN_FILE": file}))
	if err == nil || strings.Contains(err.Error(), "abc") {
		t.Fatalf("got %v", err)
	}
}

func TestPaymentsAreOffUnlessTheChainIsWired(t *testing.T) {
	file := tokenFile(t, "read,submit:secret\n")
	base := map[string]string{"SIL_API_NODE_URL": "http://127.0.0.1:8750", "SIL_API_NODE_TOKEN_FILE": file}
	off, err := Load(env(base))
	if err != nil || off.Payments {
		t.Fatalf("par défaut sans paiement : %+v, %v", off, err)
	}
	base["SIL_API_PAYMENTS"] = "true"
	on, err := Load(env(base))
	if err != nil || !on.Payments {
		t.Fatalf("SIL_API_PAYMENTS=true : %+v, %v", on, err)
	}
}
