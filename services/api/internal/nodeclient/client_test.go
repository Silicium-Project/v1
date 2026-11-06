package nodeclient

import (
	"context"
	"errors"
	"io"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

func serve(t *testing.T, handler http.HandlerFunc) *Client {
	t.Helper()
	server := httptest.NewServer(handler)
	t.Cleanup(server.Close)
	return New(server.URL+"/", "secret-token", 2*time.Second)
}

func TestContentUploadSendsBytesWithTheToken(t *testing.T) {
	var gotAuth, gotBody, gotPath string
	client := serve(t, func(w http.ResponseWriter, r *http.Request) {
		raw, _ := io.ReadAll(r.Body)
		gotAuth, gotBody, gotPath = r.Header.Get("Authorization"), string(raw), r.Method+" "+r.URL.Path
		w.WriteHeader(http.StatusCreated)
		_, _ = w.Write([]byte(`{"content_id":"ab","size":3}`))
	})
	receipt, err := client.PutContent(context.Background(), []byte("abc"))
	if err != nil {
		t.Fatal(err)
	}
	if receipt.ContentID != "ab" || gotBody != "abc" || gotAuth != "Bearer secret-token" || gotPath != "PUT /v1/content" {
		t.Fatalf("%+v %q %q %q", receipt, gotBody, gotAuth, gotPath)
	}
}

func TestStatusesMapToDistinctErrors(t *testing.T) {
	for status, want := range map[int]error{
		404: ErrNotFound, 400: ErrRejected, 401: ErrRejected,
		429: ErrUnavailable, 500: ErrUnavailable, 503: ErrUnavailable,
	} {
		client := serve(t, func(w http.ResponseWriter, r *http.Request) { w.WriteHeader(status) })
		if _, err := client.Job(context.Background(), "x"); !errors.Is(err, want) {
			t.Errorf("status %d: got %v, want %v", status, err, want)
		}
	}
}

func TestUnreachableNodeIsUnavailableAndNeverLeaksTheToken(t *testing.T) {
	client := New("http://127.0.0.1:1", "secret-token", time.Second)
	_, err := client.SubmitEvent(context.Background(), []byte("e"))
	if !errors.Is(err, ErrUnavailable) {
		t.Fatalf("got %v", err)
	}
	if contains(err.Error(), "secret-token") {
		t.Fatalf("token leaked: %v", err)
	}
}

func TestSlowNodeTimesOut(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(300 * time.Millisecond)
	}))
	defer server.Close()
	client := New(server.URL, "t", 50*time.Millisecond)
	if _, err := client.Peers(context.Background()); !errors.Is(err, ErrUnavailable) {
		t.Fatalf("got %v", err)
	}
}

func TestGarbageAnswerIsUnavailable(t *testing.T) {
	client := serve(t, func(w http.ResponseWriter, r *http.Request) { _, _ = w.Write([]byte("<html>")) })
	if _, err := client.Job(context.Background(), "x"); !errors.Is(err, ErrUnavailable) {
		t.Fatalf("got %v", err)
	}
}

func contains(haystack, needle string) bool {
	for i := 0; i+len(needle) <= len(haystack); i++ {
		if haystack[i:i+len(needle)] == needle {
			return true
		}
	}
	return false
}
