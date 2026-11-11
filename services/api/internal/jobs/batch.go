package jobs

import (
	"archive/zip"
	"bytes"
	"encoding/binary"
	"errors"
	"fmt"
)

// EmbeddingWorkload is the workload whose result is raw f32 vectors (384 per passage, in order).
const EmbeddingWorkload = "embedding/e5-small"

// Workloads whose result is a batch of files (SILBATCH frames, one file per frame).
var batchWorkloads = map[string]bool{
	"image/batch": true,
}

// IsBatch reports whether a workload's result is a batch of files.
func IsBatch(workload string) bool {
	return batchWorkloads[workload]
}

// ErrBadBatch means the archive of a batch job is not a sequence of frames.
var ErrBadBatch = errors.New("archive de lot invalide")

// BatchZip turns the archive of a batch job (frames: u32 little-endian length then the bytes,
// fragment results concatenated in order) into a ZIP with one file per frame. Each file is named
// by its position and extension, given by its first bytes.
func BatchZip(archive []byte) ([]byte, error) {
	var out bytes.Buffer
	writer := zip.NewWriter(&out)
	position := 0
	index := 0
	for position < len(archive) {
		if position+4 > len(archive) {
			return nil, ErrBadBatch
		}
		size := int(binary.LittleEndian.Uint32(archive[position : position+4]))
		position += 4
		if size > len(archive)-position {
			return nil, ErrBadBatch
		}
		file := archive[position : position+size]
		position += size
		index++
		entry, err := writer.Create(fmt.Sprintf("%04d%s", index, extensionOf(file)))
		if err != nil {
			return nil, err
		}
		if _, err := entry.Write(file); err != nil {
			return nil, err
		}
	}
	if err := writer.Close(); err != nil {
		return nil, err
	}
	return out.Bytes(), nil
}

// extensionOf names a file from its magic bytes; unknown content keeps a neutral extension.
func extensionOf(file []byte) string {
	switch {
	case bytes.HasPrefix(file, []byte{0x89, 'P', 'N', 'G'}):
		return ".png"
	case bytes.HasPrefix(file, []byte{0xff, 0xd8, 0xff}):
		return ".jpg"
	case bytes.HasPrefix(file, []byte("ID3")), len(file) > 1 && file[0] == 0xff && file[1]&0xe0 == 0xe0:
		return ".mp3"
	case bytes.HasPrefix(file, []byte("OggS")):
		return ".ogg"
	case len(file) >= 12 && string(file[:4]) == "RIFF" && string(file[8:12]) == "WAVE":
		return ".wav"
	case bytes.HasPrefix(file, []byte("fLaC")):
		return ".flac"
	}
	return ".bin"
}
