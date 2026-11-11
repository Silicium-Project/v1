package nodeclient

import (
	"context"
	"fmt"
	"io"
	"net/http"
)

// maxInput bounds what the API accepts from a node for the input of one job.
const maxInput = 64 << 20

// JobInput fetches the raw input content of a job, as the node stores it. The node answers 404 for
// an unknown job and 503 while the content is not available.
func (c *Client) JobInput(ctx context.Context, id string) ([]byte, error) {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.base+"/v1/jobs/"+id+"/input", nil)
	if err != nil {
		return nil, err
	}
	request.Header.Set("Authorization", "Bearer "+c.token)
	response, err := c.http.Do(request)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrUnavailable, redact(err, c.token))
	}
	defer response.Body.Close()
	switch {
	case response.StatusCode == http.StatusNotFound:
		return nil, ErrNotFound
	case response.StatusCode == http.StatusTooManyRequests, response.StatusCode >= 500:
		return nil, fmt.Errorf("%w: status %d", ErrUnavailable, response.StatusCode)
	case response.StatusCode >= 400:
		raw, _ := io.ReadAll(io.LimitReader(response.Body, maxResponse))
		return nil, fmt.Errorf("%w: status %d: %s", ErrRejected, response.StatusCode, firstLine(raw))
	}
	raw, err := io.ReadAll(io.LimitReader(response.Body, maxInput+1))
	if err != nil {
		return nil, fmt.Errorf("%w: reading the input", ErrUnavailable)
	}
	if len(raw) > maxInput {
		return nil, fmt.Errorf("%w: input too large", ErrUnavailable)
	}
	return raw, nil
}
