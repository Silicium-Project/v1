package jobs

import (
	"bytes"
	"context"
	"crypto/sha256"
	"fmt"
	"image/png"
	"strings"
	"testing"

	"sil/api/internal/nodeclient"
)

// renderJob is what the node says about a finished render job.
const renderJob = `{"status":"Finalized","workload":"rendu/raytracer","image":true,"tasks":[]}`

// shaJob is what the node says about a finished sha256 job.
const shaJob = `{"status":"Finalized","workload":"calcul/sha256","image":false,"tasks":[]}`

func TestRenderResultIsAPNGOfTheOwnersJob(t *testing.T) {
	svc, node := setup()
	node.job = renderJob
	if _, err := svc.Publish(context.Background(), "alice", id, []byte("event"), "render"); err != nil {
		t.Fatal(err)
	}
	result, err := svc.Result(context.Background(), "alice", id)
	if err != nil {
		t.Fatal(err)
	}
	if result.ContentType != "image/png" || !strings.HasSuffix(result.Filename, ".png") {
		t.Fatalf("content %s file %s", result.ContentType, result.Filename)
	}
	decoded, err := png.Decode(bytes.NewReader(result.Body))
	if err != nil {
		t.Fatalf("not a PNG: %v", err)
	}
	if decoded.Bounds().Dx() != 2 || decoded.Bounds().Dy() != 1 {
		t.Fatalf("size %v", decoded.Bounds())
	}
	r, g, b, a := decoded.At(0, 0).RGBA()
	if r>>8 != 255 || g>>8 != 0 || b>>8 != 0 || a>>8 != 255 {
		t.Fatalf("first pixel %d %d %d %d", r>>8, g>>8, b>>8, a>>8)
	}
}

func TestShaResultIsATextFileWithEachFragmentAndTheFinalDigest(t *testing.T) {
	svc, node := setup()
	node.job = shaJob
	if _, err := svc.Publish(context.Background(), "alice", id, []byte("event"), "sha"); err != nil {
		t.Fatal(err)
	}
	result, err := svc.Result(context.Background(), "alice", id)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(result.ContentType, "text/plain") || !strings.HasSuffix(result.Filename, ".txt") {
		t.Fatalf("content %s file %s", result.ContentType, result.Filename)
	}
	text := string(result.Body)
	final := sha256.Sum256([]byte{0xab, 0xcd, 0x01})
	for _, line := range []string{
		"workload: calcul/sha256",
		"fragment 0: abcd",
		"fragment 1: 01",
		fmt.Sprintf("%x", final),
	} {
		if !strings.Contains(text, line) {
			t.Fatalf("missing %q in\n%s", line, text)
		}
	}
}

// embeddingJob is what the node says about a finished embedding job.
const embeddingJob = `{"status":"Finalized","workload":"embedding/e5-small","image":false,"tasks":[]}`

func TestEmbeddingResultIsTheRawVectorsNotTheHexOutputs(t *testing.T) {
	svc, node := setup()
	node.job = embeddingJob
	if _, err := svc.Publish(context.Background(), "alice", id, []byte("event"), "embedding"); err != nil {
		t.Fatal(err)
	}
	result, err := svc.Result(context.Background(), "alice", id)
	if err != nil {
		t.Fatal(err)
	}
	if result.ContentType != "application/octet-stream" || !strings.HasSuffix(result.Filename, ".bin") {
		t.Fatalf("content %s file %s", result.ContentType, result.Filename)
	}
	if want := []byte{0x04, 0x00, 0x00, 0x00, 0x89, 'P', 'N', 'G'}; !bytes.Equal(result.Body, want) {
		t.Fatalf("body %x, want the archive of the fragments", result.Body)
	}
}

func TestAnotherUsersJobHasNoResult(t *testing.T) {
	svc, node := setup()
	node.job = renderJob
	if _, err := svc.Publish(context.Background(), "alice", id, []byte("event"), "render"); err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Result(context.Background(), "mallory", id); err != ErrNotFound {
		t.Fatalf("want ErrNotFound, got %v", err)
	}
}

func TestEncodePNGKeepsPixels(t *testing.T) {
	picture, err := EncodePNG(nodeclient.Image{Width: 1, Height: 1, RGB: []byte{1, 2, 3}})
	if err != nil {
		t.Fatal(err)
	}
	decoded, err := png.Decode(bytes.NewReader(picture))
	if err != nil {
		t.Fatal(err)
	}
	r, g, b, _ := decoded.At(0, 0).RGBA()
	if r>>8 != 1 || g>>8 != 2 || b>>8 != 3 {
		t.Fatalf("pixel %d %d %d", r>>8, g>>8, b>>8)
	}
}
