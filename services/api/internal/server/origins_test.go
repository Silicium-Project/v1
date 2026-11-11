package server

import (
	"encoding/base64"
	"encoding/json"
	"net/http"
	"strings"
	"testing"
)

func TestOriginsAreWrittenAndReadOnlyByTheOwnerOfTheJob(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "frank@example.com")
	event := base64.StdEncoding.EncodeToString([]byte("e"))
	if status, _ := e.call(t, "POST", "/v1/jobs", token, map[string]string{"job_id": jobHex, "event": event, "label": "embedding"}); status != 201 {
		t.Fatalf("publish: %d", status)
	}

	if status, _ := e.call(t, "PUT", "/v1/jobs/"+jobHex+"/origins", token, map[string][]string{"origins": {"rapport.pdf · page 3", "notes.txt · ligne 4"}}); status != http.StatusOK {
		t.Fatalf("put origins: %d", status)
	}
	status, _, body := raw(t, e, "/v1/jobs/"+jobHex+"/origins", token)
	var got struct {
		Origins []string `json:"origins"`
	}
	if status != http.StatusOK || json.Unmarshal(body, &got) != nil || len(got.Origins) != 2 || got.Origins[0] != "rapport.pdf · page 3" {
		t.Fatalf("get origins: %d %s", status, body)
	}

	other := e.login(t, "grace@example.com")
	if status, _, _ := raw(t, e, "/v1/jobs/"+jobHex+"/origins", other); status != http.StatusNotFound {
		t.Fatalf("another user read the origins: %d", status)
	}
	if status, _ := e.call(t, "PUT", "/v1/jobs/"+jobHex+"/origins", other, map[string][]string{"origins": {"x"}}); status != http.StatusNotFound {
		t.Fatalf("another user wrote the origins: %d", status)
	}
	if status, _, _ := raw(t, e, "/v1/jobs/"+jobHex+"/origins", ""); status != http.StatusUnauthorized {
		t.Fatalf("no token: %d", status)
	}
}

func TestOriginsAreRefusedWhenTooLong(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "frank@example.com")
	event := base64.StdEncoding.EncodeToString([]byte("e"))
	if status, _ := e.call(t, "POST", "/v1/jobs", token, map[string]string{"job_id": jobHex, "event": event, "label": "embedding"}); status != 201 {
		t.Fatalf("publish: %d", status)
	}
	long := strings.Repeat("a", 300)
	if status, _ := e.call(t, "PUT", "/v1/jobs/"+jobHex+"/origins", token, map[string][]string{"origins": {long}}); status != http.StatusBadRequest {
		t.Fatalf("long origin: %d", status)
	}
}
