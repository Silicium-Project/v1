// Package nodeclient talks to a Silicium node through its control API (sil-control-api).
// The node never signs for the user: this client only forwards what the user signed.
package nodeclient

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Errors a caller can act on.
var (
	// ErrUnavailable: the node did not answer or answered 5xx/429. The state of the request
	// is unknown; callers must report "unknown", never "failed" or "done".
	ErrUnavailable = errors.New("node unavailable")
	// ErrRejected: the node understood and refused (4xx other than 404/429).
	ErrRejected = errors.New("node rejected the request")
	// ErrNotFound: the node does not know this job.
	ErrNotFound = errors.New("not found on this node")
)

const maxResponse = 1 << 20 // the node answers with small JSON documents

// Client is safe for concurrent use.
type Client struct {
	base  string
	token string
	http  *http.Client
}

// New builds a client for the node at base (for example http://127.0.0.1:8750).
func New(base, token string, timeout time.Duration) *Client {
	return &Client{
		base:  strings.TrimRight(base, "/"),
		token: token,
		http:  &http.Client{Timeout: timeout},
	}
}

// ContentReceipt is the node answer to a content upload.
type ContentReceipt struct {
	ContentID string `json:"content_id"`
	Size      int    `json:"size"`
}

// EventReceipt is the node answer to an event submission.
type EventReceipt struct {
	Accepted int  `json:"accepted"`
	Pending  bool `json:"pending"`
}

// PutContent stores bytes on the node and returns their content id.
func (c *Client) PutContent(ctx context.Context, data []byte) (ContentReceipt, error) {
	var out ContentReceipt
	err := c.do(ctx, http.MethodPut, "/v1/content", data, &out)
	return out, err
}

// SubmitEvent sends one signed, canonically encoded event.
func (c *Client) SubmitEvent(ctx context.Context, event []byte) (EventReceipt, error) {
	var out EventReceipt
	err := c.do(ctx, http.MethodPost, "/v1/events", event, &out)
	return out, err
}

// Job returns the derived state of a job as the node sees it (raw JSON).
func (c *Client) Job(ctx context.Context, id string) (json.RawMessage, error) {
	var out json.RawMessage
	err := c.do(ctx, http.MethodGet, "/v1/jobs/"+id, nil, &out)
	return out, err
}

// Peers returns the node peer list (raw JSON).
func (c *Client) Peers(ctx context.Context) (json.RawMessage, error) {
	var out json.RawMessage
	err := c.do(ctx, http.MethodGet, "/v1/peers", nil, &out)
	return out, err
}

func (c *Client) do(ctx context.Context, method, path string, body []byte, out any) error {
	request, err := http.NewRequestWithContext(ctx, method, c.base+path, bytes.NewReader(body))
	if err != nil {
		return err
	}
	request.Header.Set("Authorization", "Bearer "+c.token)
	if body != nil {
		request.Header.Set("Content-Type", "application/octet-stream")
	}
	response, err := c.http.Do(request)
	if err != nil {
		return fmt.Errorf("%w: %v", ErrUnavailable, redact(err, c.token))
	}
	defer response.Body.Close()
	raw, err := io.ReadAll(io.LimitReader(response.Body, maxResponse))
	if err != nil {
		return fmt.Errorf("%w: reading the answer", ErrUnavailable)
	}
	switch {
	case response.StatusCode == http.StatusNotFound:
		return ErrNotFound
	case response.StatusCode == http.StatusTooManyRequests, response.StatusCode >= 500:
		return fmt.Errorf("%w: status %d", ErrUnavailable, response.StatusCode)
	case response.StatusCode >= 400:
		return fmt.Errorf("%w: status %d: %s", ErrRejected, response.StatusCode, firstLine(raw))
	}
	if err := json.Unmarshal(raw, out); err != nil {
		return fmt.Errorf("%w: unreadable answer", ErrUnavailable)
	}
	return nil
}

func redact(err error, token string) string {
	if token == "" {
		return err.Error()
	}
	return strings.ReplaceAll(err.Error(), token, "[redacted]")
}

func firstLine(raw []byte) string {
	text := strings.TrimSpace(string(raw))
	if i := strings.IndexByte(text, '\n'); i >= 0 {
		text = text[:i]
	}
	if len(text) > 200 {
		text = text[:200]
	}
	return text
}
