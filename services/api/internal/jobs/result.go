package jobs

import (
	"bytes"
	"context"
	"crypto/sha256"
	"fmt"
	"image"
	"image/png"

	"sil/api/internal/nodeclient"
)

// Result is what a finished job gives to download: a PNG picture for a render job, a text file
// with the raw fragment results otherwise.
type Result struct {
	ContentType string
	Filename    string
	Body        []byte
}

// Result returns the finished result of a job of the user.
func (s *Service) Result(ctx context.Context, userID, id string) (Result, error) {
	if !jobID.MatchString(id) {
		return Result{}, ErrInvalidInput
	}
	if _, err := s.store.JobOf(userID, id); err != nil {
		return Result{}, ErrNotFound
	}
	raw, err := s.node.Job(ctx, id)
	if err != nil {
		return Result{}, err
	}
	status := Interpret(raw, nil)
	if status.Image {
		img, err := s.node.JobImage(ctx, id)
		if err != nil {
			return Result{}, err
		}
		picture, err := EncodePNG(img)
		if err != nil {
			return Result{}, err
		}
		return Result{ContentType: "image/png", Filename: "silicium-" + id[:8] + ".png", Body: picture}, nil
	}
	if IsBatch(status.Workload) {
		archive, err := s.node.JobArchive(ctx, id)
		if err != nil {
			return Result{}, err
		}
		zipped, err := BatchZip(archive)
		if err != nil {
			return Result{}, err
		}
		return Result{ContentType: "application/zip", Filename: "silicium-" + id[:8] + ".zip", Body: zipped}, nil
	}
	// Embeddings : un vecteur de 1 536 octets par passage, trop gros pour la sortie par fragment en
	// hexadécimal. Le résultat brut (vecteurs dans l'ordre des fragments) est donné tel quel.
	if status.Workload == EmbeddingWorkload {
		archive, err := s.node.JobArchive(ctx, id)
		if err != nil {
			return Result{}, err
		}
		return Result{ContentType: "application/octet-stream", Filename: "silicium-" + id[:8] + ".bin", Body: archive}, nil
	}
	outputs, err := s.node.JobOutputs(ctx, id)
	if err != nil {
		return Result{}, err
	}
	return Result{
		ContentType: "text/plain; charset=utf-8",
		Filename:    "silicium-" + id[:8] + ".txt",
		Body:        textResult(id, status.Workload, outputs),
	}, nil
}

// textResult lists the raw result of each fragment, then the final result: the sha256 of the
// fragment results taken in order, which is what the sha256 workload combines.
func textResult(id, workload string, outputs []nodeclient.Output) []byte {
	var buf bytes.Buffer
	fmt.Fprintf(&buf, "Silicium, résultat du calcul %s\n", id)
	fmt.Fprintf(&buf, "workload: %s\n", workload)
	var joined []byte
	for _, output := range outputs {
		fmt.Fprintf(&buf, "fragment %d: %x\n", output.Fragment, output.Bytes)
		joined = append(joined, output.Bytes...)
	}
	final := sha256.Sum256(joined)
	fmt.Fprintf(&buf, "résultat final (sha256 des fragments, dans l'ordre): %x\n", final)
	return buf.Bytes()
}

// EncodePNG turns RGB24 rows into a PNG file with opaque pixels.
func EncodePNG(img nodeclient.Image) ([]byte, error) {
	rgba := image.NewNRGBA(image.Rect(0, 0, img.Width, img.Height))
	for pixel := 0; pixel < img.Width*img.Height; pixel++ {
		rgba.Pix[pixel*4] = img.RGB[pixel*3]
		rgba.Pix[pixel*4+1] = img.RGB[pixel*3+1]
		rgba.Pix[pixel*4+2] = img.RGB[pixel*3+2]
		rgba.Pix[pixel*4+3] = 0xff
	}
	var buf bytes.Buffer
	if err := png.Encode(&buf, rgba); err != nil {
		return nil, err
	}
	return buf.Bytes(), nil
}
