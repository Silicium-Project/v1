package server

import (
	"io"
	"net/http"
	"os"
	"path/filepath"
	"testing"
)

func TestModelFilesAreServedWithALongCacheAndTheirFullBody(t *testing.T) {
	dir := t.TempDir()
	models := filepath.Join(dir, "download", "models", "e5")
	if err := os.MkdirAll(models, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(models, "model.onnx"), []byte("poids"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "index.html"), []byte("<html></html>"), 0o644); err != nil {
		t.Fatal(err)
	}
	e := newEnvWeb(t, dir)

	response, err := http.Get(e.api.URL + "/download/models/e5/model.onnx")
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	body, _ := io.ReadAll(response.Body)
	if response.StatusCode != http.StatusOK || string(body) != "poids" {
		t.Fatalf("status %d body %q", response.StatusCode, body)
	}
	if got := response.Header.Get("Cache-Control"); got != "public, max-age=604800" {
		t.Fatalf("cache-control %q", got)
	}
}
