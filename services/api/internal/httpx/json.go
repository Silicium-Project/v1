// Package httpx holds the small HTTP helpers shared by every handler: JSON in and out,
// uniform errors, request limits and middleware.
package httpx

import (
	"encoding/json"
	"errors"
	"io"
	"net/http"
)

// ErrorBody is the only error shape the API returns.
type ErrorBody struct {
	Code    string `json:"code"`
	Message string `json:"message"`
}

// WriteJSON writes v with the given status.
func WriteJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(v)
}

// WriteError writes a uniform error. The message must never contain secrets.
func WriteError(w http.ResponseWriter, status int, code, message string) {
	WriteJSON(w, status, ErrorBody{Code: code, Message: message})
}

// ReadJSON decodes a body of at most max bytes into dst, refusing unknown fields and
// trailing data.
func ReadJSON(w http.ResponseWriter, r *http.Request, dst any, max int64) error {
	decoder := json.NewDecoder(http.MaxBytesReader(w, r.Body, max))
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(dst); err != nil {
		return err
	}
	if _, err := decoder.Token(); !errors.Is(err, io.EOF) {
		return errors.New("unexpected data after the JSON value")
	}
	return nil
}

// ReadBody reads at most max bytes; a larger body is an error.
func ReadBody(w http.ResponseWriter, r *http.Request, max int64) ([]byte, error) {
	return io.ReadAll(http.MaxBytesReader(w, r.Body, max))
}

// IsTooLarge tells whether err comes from a body over the limit.
func IsTooLarge(err error) bool {
	var tooLarge *http.MaxBytesError
	return errors.As(err, &tooLarge)
}
