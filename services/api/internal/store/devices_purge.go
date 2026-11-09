package store

import "errors"

// ErrNotRevoked is returned when a device must be revoked before it can be deleted.
var ErrNotRevoked = errors.New("device is still active")

// PurgeRevokedDevice deletes a revoked device of this user for good. An active device is refused:
// it has to be revoked first. The device record goes; the ledger history of its key stays.
func (s *Store) PurgeRevokedDevice(userID, id string) error {
	return s.update(func(d *snapshot) error {
		device, ok := d.Devices[id]
		if !ok || device.UserID != userID {
			return ErrNotFound
		}
		if device.RevokedAt == nil {
			return ErrNotRevoked
		}
		delete(d.Devices, id)
		return nil
	})
}
