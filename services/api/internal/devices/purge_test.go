package devices

import (
	"errors"
	"testing"

	"sil/api/internal/store"
)

func TestOnlyARevokedDeviceCanBePurged(t *testing.T) {
	svc, _ := setup(t)
	code, _, err := svc.NewCode("user-1")
	if err != nil {
		t.Fatal(err)
	}
	pub, sig := sign(vectorKey(), code)
	device, err := svc.Link(code, pub, sig, "pc")
	if err != nil {
		t.Fatal(err)
	}

	if err := svc.Purge("user-1", device.ID); !errors.Is(err, store.ErrNotRevoked) {
		t.Fatalf("active device purged: %v", err)
	}
	if err := svc.Purge("user-2", device.ID); !errors.Is(err, store.ErrNotFound) {
		t.Fatalf("another account's device: %v", err)
	}
	if err := svc.Revoke("user-1", device.ID); err != nil {
		t.Fatal(err)
	}
	if err := svc.Purge("user-1", "unknown"); !errors.Is(err, store.ErrNotFound) {
		t.Fatalf("unknown device: %v", err)
	}
	if err := svc.Purge("user-1", device.ID); err != nil {
		t.Fatalf("revoked device not purged: %v", err)
	}
	if left := svc.List("user-1"); len(left) != 0 {
		t.Fatalf("still listed after purge: %+v", left)
	}
}
