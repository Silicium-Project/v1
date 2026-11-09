package server

import (
	"bytes"
	"encoding/base64"
	"image/png"
	"io"
	"net/http"
	"testing"
)

// raw fetches a path and returns the status, the content type and the body bytes.
func raw(t *testing.T, e *env, path, token string) (int, string, []byte) {
	t.Helper()
	request, _ := http.NewRequest(http.MethodGet, e.api.URL+path, nil)
	if token != "" {
		request.Header.Set("Authorization", "Bearer "+token)
	}
	response, err := http.DefaultClient.Do(request)
	if err != nil {
		t.Fatal(err)
	}
	defer response.Body.Close()
	body, _ := io.ReadAll(response.Body)
	return response.StatusCode, response.Header.Get("Content-Type"), body
}

func TestFinishedRenderIsDownloadableAsPNGByItsOwnerOnly(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "frank@example.com")
	event := base64.StdEncoding.EncodeToString([]byte("e"))
	if status, _ := e.call(t, "POST", "/v1/jobs", token, map[string]string{"job_id": jobHex, "event": event, "label": "render"}); status != 201 {
		t.Fatalf("publish: %d", status)
	}

	status, contentType, body := raw(t, e, "/v1/jobs/"+jobHex+"/result", token)
	if status != http.StatusOK || contentType != "image/png" {
		t.Fatalf("result: %d %s", status, contentType)
	}
	decoded, err := png.Decode(bytes.NewReader(body))
	if err != nil || decoded.Bounds().Dx() != 2 || decoded.Bounds().Dy() != 1 {
		t.Fatalf("not the 2x1 PNG: %v %v", err, decoded)
	}

	other := e.login(t, "grace@example.com")
	if status, _, _ := raw(t, e, "/v1/jobs/"+jobHex+"/result", other); status != http.StatusNotFound {
		t.Fatalf("another user got the result: %d", status)
	}
	if status, _, _ := raw(t, e, "/v1/jobs/"+jobHex+"/result", ""); status != http.StatusUnauthorized {
		t.Fatalf("no token: %d", status)
	}
}
