package nodeclient

import (
	"context"
	"encoding/json"
	"net/http"
)

// SubmitSettlement sends a settlement signed by the approvers (canonical CBOR) to a node that
// pays the transaction fees. The node checks the signatures and the chain; the answer carries the
// transaction id ({"tx": ...}), which is not a confirmation: the chain confirms it.
func (c *Client) SubmitSettlement(ctx context.Context, settlement []byte) (json.RawMessage, error) {
	var out json.RawMessage
	err := c.do(ctx, http.MethodPost, "/v1/settlements", settlement, &out)
	return out, err
}
