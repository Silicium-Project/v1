package jobs

import (
	"archive/zip"
	"bytes"
	"encoding/binary"
	"io"
	"testing"
)

func frameOf(parts ...[]byte) []byte {
	var out []byte
	for _, part := range parts {
		var size [4]byte
		binary.LittleEndian.PutUint32(size[:], uint32(len(part)))
		out = append(out, size[:]...)
		out = append(out, part...)
	}
	return out
}

func TestBatchZipKeepsEveryFileInOrderWithItsExtension(t *testing.T) {
	png := []byte{0x89, 'P', 'N', 'G', 1, 2}
	jpeg := []byte{0xff, 0xd8, 0xff, 0xe0, 9}
	zipped, err := BatchZip(frameOf(png, []byte("texte"), jpeg))
	if err != nil {
		t.Fatalf("BatchZip: %v", err)
	}
	reader, err := zip.NewReader(bytes.NewReader(zipped), int64(len(zipped)))
	if err != nil {
		t.Fatalf("zip: %v", err)
	}
	want := map[string][]byte{"0001.png": png, "0002.bin": []byte("texte"), "0003.jpg": jpeg}
	if len(reader.File) != len(want) {
		t.Fatalf("entries = %d, want %d", len(reader.File), len(want))
	}
	for _, file := range reader.File {
		content, err := file.Open()
		if err != nil {
			t.Fatalf("open %s: %v", file.Name, err)
		}
		data, _ := io.ReadAll(content)
		content.Close()
		if !bytes.Equal(data, want[file.Name]) {
			t.Fatalf("%s content differs", file.Name)
		}
	}
}

func TestBatchZipRefusesAFrameThatPastsItsEnd(t *testing.T) {
	broken := frameOf([]byte("abc"))
	if _, err := BatchZip(broken[:len(broken)-1]); err != ErrBadBatch {
		t.Fatalf("err = %v, want ErrBadBatch", err)
	}
}

func TestBatchWorkloadsAreTheTwoFileBatches(t *testing.T) {
	if !IsBatch("image/batch") {
		t.Fatal("batch workloads not recognised")
	}
	if IsBatch("calcul/sha256") || IsBatch("rendu/raytracer") {
		t.Fatal("non-batch workload recognised as batch")
	}
}
