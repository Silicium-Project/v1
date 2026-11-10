package store

import "errors"

// ErrStale means a telemetry report is not newer than the one already kept for the machine.
var ErrStale = errors.New("stale telemetry")

// SetTelemetry keeps the report of the active device that holds this public key. An older report
// than the kept one is refused, so a replayed message cannot roll the state back. A revoked or
// unknown key gets ErrNotFound.
func (s *Store) SetTelemetry(publicKey string, report Telemetry) error {
	return s.update(func(d *snapshot) error {
		for id, device := range d.Devices {
			if device.PublicKey != publicKey || device.RevokedAt != nil {
				continue
			}
			if device.Telemetry != nil && !report.ObservedAt.After(device.Telemetry.ObservedAt) {
				return ErrStale
			}
			device.Telemetry = &report
			d.Devices[id] = device
			return nil
		}
		return ErrNotFound
	})
}
