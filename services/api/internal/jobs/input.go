package jobs

import "context"

// Input returns the input content of a job of the user, as the node stores it. Only the owner of
// the job reads it: another user gets ErrNotFound.
func (s *Service) Input(ctx context.Context, userID, id string) ([]byte, error) {
	if !jobID.MatchString(id) {
		return nil, ErrInvalidInput
	}
	if _, err := s.store.JobOf(userID, id); err != nil {
		return nil, ErrNotFound
	}
	return s.node.JobInput(ctx, id)
}
