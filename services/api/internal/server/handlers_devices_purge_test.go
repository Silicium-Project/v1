package server

import (
	"bytes"
	"crypto/ed25519"
	"encoding/hex"
	"testing"

	"sil/api/internal/devices"
)

// A machine can be deleted for good only once it is revoked; the list then no longer shows it.
func TestRevokedDeviceCanBePurgedAndActiveOneCannot(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "alice@example.com")
	_, out := e.call(t, "POST", "/v1/devices/link-codes", token, nil)
	code := out["code"].(string)
	key := ed25519.NewKeyFromSeed(bytes.Repeat([]byte{7}, 32))
	pub := key.Public().(ed25519.PublicKey)
	sig := ed25519.Sign(key, devices.Frame(devices.Message(code, pub)))
	status, device := e.call(t, "POST", "/v1/devices/link", "", map[string]string{
		"code": code, "public_key": hex.EncodeToString(pub), "signature": hex.EncodeToString(sig), "label": "pc",
	})
	if status != 201 {
		t.Fatalf("link: %d %v", status, device)
	}
	id := device["id"].(string)

	if status, out := e.call(t, "DELETE", "/v1/devices/"+id+"/purge", token, nil); status != 409 || out["code"] != "device_active" {
		t.Fatalf("purge of an active device: %d %v", status, out)
	}
	if status, _ := e.call(t, "DELETE", "/v1/devices/"+id, token, nil); status != 204 {
		t.Fatalf("revoke: %d", status)
	}
	if status, _ := e.call(t, "DELETE", "/v1/devices/"+id+"/purge", token, nil); status != 204 {
		t.Fatalf("purge: %d", status)
	}
	if _, list := e.call(t, "GET", "/v1/devices", token, nil); len(list["devices"].([]any)) != 0 {
		t.Fatalf("still listed: %v", list)
	}
}
