package devices

// Purge deletes a revoked device of the user, for good. An active device is refused with
// store.ErrNotRevoked: revoke it first.
func (s *Service) Purge(userID, deviceID string) error {
	return s.store.PurgeRevokedDevice(userID, deviceID)
}
