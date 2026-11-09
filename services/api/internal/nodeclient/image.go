package nodeclient

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"strconv"
)

// Image is the assembled picture of a finished render job: RGB24 rows, top to bottom.
type Image struct {
	Width  int
	Height int
	RGB    []byte
}

// maxImage bounds what the API accepts from a node for one picture.
const maxImage = 256 << 20

// JobImage fetches the assembled image of a finished render job. The node answers 503 while
// some fragment is not final yet, 400 for a workload that does not produce an image.
func (c *Client) JobImage(ctx context.Context, id string) (Image, error) {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, c.base+"/v1/jobs/"+id+"/image", nil)
	if err != nil {
		return Image{}, err
	}
	request.Header.Set("Authorization", "Bearer "+c.token)
	response, err := c.http.Do(request)
	if err != nil {
		return Image{}, fmt.Errorf("%w: %v", ErrUnavailable, redact(err, c.token))
	}
	defer response.Body.Close()
	switch {
	case response.StatusCode == http.StatusNotFound:
		return Image{}, ErrNotFound
	case response.StatusCode == http.StatusTooManyRequests, response.StatusCode >= 500:
		return Image{}, fmt.Errorf("%w: status %d", ErrUnavailable, response.StatusCode)
	case response.StatusCode >= 400:
		raw, _ := io.ReadAll(io.LimitReader(response.Body, maxResponse))
		return Image{}, fmt.Errorf("%w: status %d: %s", ErrRejected, response.StatusCode, firstLine(raw))
	}
	width, errWidth := strconv.Atoi(response.Header.Get("X-Image-Width"))
	height, errHeight := strconv.Atoi(response.Header.Get("X-Image-Height"))
	if errWidth != nil || errHeight != nil || width <= 0 || height <= 0 {
		return Image{}, fmt.Errorf("%w: image size missing", ErrUnavailable)
	}
	rgb, err := io.ReadAll(io.LimitReader(response.Body, maxImage+1))
	if err != nil {
		return Image{}, fmt.Errorf("%w: reading the image", ErrUnavailable)
	}
	if len(rgb) > maxImage || len(rgb) != width*height*3 {
		return Image{}, fmt.Errorf("%w: image size mismatch", ErrUnavailable)
	}
	return Image{Width: width, Height: height, RGB: rgb}, nil
}
