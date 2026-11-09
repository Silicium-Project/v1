package nodeclient

import (
	"context"
	"encoding/hex"
	"fmt"
	"net/http"
)

// Output is the raw result of one fragment of a finished job.
type Output struct {
	Fragment int
	Bytes    []byte
}

// JobOutputs fetches the raw result of each fragment of a finished job, in fragment order. The
// node answers 503 while some fragment is not final yet, 400 for a result too large for the API.
func (c *Client) JobOutputs(ctx context.Context, id string) ([]Output, error) {
	var answer struct {
		Fragments []struct {
			Fragment int    `json:"fragment"`
			Hex      string `json:"hex"`
		} `json:"fragments"`
	}
	if err := c.do(ctx, http.MethodGet, "/v1/jobs/"+id+"/outputs", nil, &answer); err != nil {
		return nil, err
	}
	outputs := make([]Output, 0, len(answer.Fragments))
	for _, fragment := range answer.Fragments {
		raw, err := hex.DecodeString(fragment.Hex)
		if err != nil {
			return nil, fmt.Errorf("%w: unreadable output of fragment %d", ErrUnavailable, fragment.Fragment)
		}
		outputs = append(outputs, Output{Fragment: fragment.Fragment, Bytes: raw})
	}
	return outputs, nil
}
