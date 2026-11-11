// Package config reads and validates the service configuration. It is the only place that
// reads environment variables.
package config

import (
	"bufio"
	"errors"
	"fmt"
	"net/url"
	"os"
	"strconv"
	"strings"
	"time"
)

// Config is the validated configuration.
type Config struct {
	Addr              string        // listen address
	StorePath         string        // SQLite database file (store.json next to it is imported once)
	NodeURL           string        // control API of the node
	NodeToken         string        // token with the read and submit scopes (never logged)
	NodeTimeout       time.Duration // per request
	AllowRegistration bool
	SessionTTL        time.Duration
	LogJSON           bool
	WebDir            string   // built web app to serve (empty: API only)
	ConnectSources    []string // extra origins the web app may call (Solana RPC for the deposit)
	Payments          bool     // jobs are settled on a chain (SIL_API_PAYMENTS=true)
	CORSOrigins       []string // origins allowed to call the API (desktop shell); empty: none
}

// Load reads SIL_API_* variables through getenv (os.Getenv in production).
func Load(getenv func(string) string) (Config, error) {
	cfg := Config{
		Addr:              valueOr(getenv("SIL_API_ADDR"), "127.0.0.1:8080"),
		StorePath:         valueOr(getenv("SIL_API_STORE"), "data/silicium.db"),
		NodeURL:           strings.TrimSpace(getenv("SIL_API_NODE_URL")),
		NodeTimeout:       10 * time.Second,
		AllowRegistration: true,
		SessionTTL:        24 * time.Hour,
		LogJSON:           getenv("SIL_API_LOG_JSON") == "true",
		WebDir:            strings.TrimSpace(getenv("SIL_API_WEB_DIR")),
		ConnectSources:    splitList(getenv("SIL_API_CONNECT_SRC")),
		Payments:          getenv("SIL_API_PAYMENTS") == "true",
		CORSOrigins:       splitList(getenv("SIL_API_CORS_ORIGINS")),
	}
	var problems []error
	if v := getenv("SIL_API_ALLOW_REGISTRATION"); v != "" {
		b, err := strconv.ParseBool(v)
		if err != nil {
			problems = append(problems, fmt.Errorf("SIL_API_ALLOW_REGISTRATION: %q is not a boolean", v))
		}
		cfg.AllowRegistration = b
	}
	if v := getenv("SIL_API_SESSION_HOURS"); v != "" {
		hours, err := strconv.Atoi(v)
		if err != nil || hours < 1 || hours > 24*30 {
			problems = append(problems, fmt.Errorf("SIL_API_SESSION_HOURS: want 1 to 720, got %q", v))
		} else {
			cfg.SessionTTL = time.Duration(hours) * time.Hour
		}
	}
	if u, err := url.Parse(cfg.NodeURL); err != nil || (u.Scheme != "http" && u.Scheme != "https") || u.Host == "" {
		problems = append(problems, errors.New("SIL_API_NODE_URL: an http(s) URL is required"))
	}
	token, err := nodeToken(getenv("SIL_API_NODE_TOKEN_FILE"))
	if err != nil {
		problems = append(problems, err)
	}
	cfg.NodeToken = token
	return cfg, errors.Join(problems...)
}

func valueOr(value, fallback string) string {
	if value = strings.TrimSpace(value); value != "" {
		return value
	}
	return fallback
}

// nodeToken reads the node token file (same format as the node: "scope1,scope2:secret"
// lines, "#" for comments) and returns the secret of the line that has both scopes.
func nodeToken(path string) (string, error) {
	if path == "" {
		return "", errors.New("SIL_API_NODE_TOKEN_FILE: required")
	}
	file, err := os.Open(path)
	if err != nil {
		return "", fmt.Errorf("SIL_API_NODE_TOKEN_FILE: %w", err)
	}
	defer file.Close()
	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		scopes, secret, found := strings.Cut(line, ":")
		if !found || strings.HasPrefix(line, "#") {
			continue
		}
		have := map[string]bool{}
		for _, scope := range strings.Split(scopes, ",") {
			have[strings.TrimSpace(scope)] = true
		}
		if have["read"] && have["submit"] && secret != "" {
			return secret, nil
		}
	}
	return "", errors.New("SIL_API_NODE_TOKEN_FILE: no line with the scopes read and submit")
}

func splitList(value string) []string {
	var out []string
	for _, item := range strings.Split(value, ",") {
		if item = strings.TrimSpace(item); item != "" {
			out = append(out, item)
		}
	}
	return out
}

func isHex(text string) bool {
	for _, c := range text {
		if !(c >= '0' && c <= '9' || c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}
