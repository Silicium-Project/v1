package jobs

import (
	"errors"
	"unicode/utf8"

	"sil/api/internal/store"
)

// Limits on the origins of a job: one short text per passage (file, page, line).
const (
	MaxOrigins      = 20_000
	maxOriginRunes  = 256
	maxOriginsBytes = 2 << 20
)

// SetOrigins records where each passage of an embedding job comes from. Only the owner of the job
// can set them; the texts stay with the job, they are not part of the job list.
func (s *Service) SetOrigins(userID, id string, origins []string) error {
	if !jobID.MatchString(id) || len(origins) > MaxOrigins {
		return ErrInvalidInput
	}
	total := 0
	for _, origin := range origins {
		if utf8.RuneCountInString(origin) > maxOriginRunes {
			return ErrInvalidInput
		}
		total += len(origin)
	}
	if total > maxOriginsBytes {
		return ErrInvalidInput
	}
	if err := s.store.SetOrigins(userID, id, origins); err != nil {
		if errors.Is(err, store.ErrNotFound) {
			return ErrNotFound
		}
		return err
	}
	return nil
}

// Origins returns the origins of a job of the user.
func (s *Service) Origins(userID, id string) ([]string, error) {
	if !jobID.MatchString(id) {
		return nil, ErrInvalidInput
	}
	origins, err := s.store.OriginsOf(userID, id)
	if err != nil {
		return nil, ErrNotFound
	}
	if origins == nil {
		origins = []string{}
	}
	return origins, nil
}
