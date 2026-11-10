package nodeclient

import (
	"context"
	"fmt"
	"io"
	"net/http"
)

// maxArchive bounds what the API accepts from a node for the results of one batch job.
const maxArchive = 64 << 20

// JobArchive fetches the raw results of every fragment of a finished batch job, concatenated in
// fragment order (each fragment result is a frame, so the concatenation is a frame too). The node
// answers 503 while a fragment is not final yet, 400 when the result is too large.
func (c *Client) JobArchive(ctx context.Context, id string) ([]byte, error) {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.base+"/v1/jobs/"+id+"/archive", nil)
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
	raw, err := io.ReadAll(io.LimitReader(response.Body, maxArchive+1))
	if err != nil {
		return nil, fmt.Errorf("%w: reading the archive", ErrUnavailable)
	}
	if len(raw) > maxArchive {
		return nil, fmt.Errorf("%w: archive too large", ErrUnavailable)
	}
	return raw, nil
}
