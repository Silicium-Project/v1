package devices

import (
	"crypto/ed25519"
	"encoding/hex"
	"errors"
	"testing"
	"time"

	"sil/api/internal/store"
)

// Test vector shared with contracts/device-link.md and the Rust test in sil-crypto.
const (
	vectorSeed      = "0707070707070707070707070707070707070707070707070707070707070707"
	vectorCode      = "ABCDEFGHJKLM"
	vectorPublic    = "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c"
	vectorSignature = "146c5c2c78b2a2bf29904f13fe926374ee5f343034ae2fdc763943c45ed00e714a0b892b48ecab33df2c1ef8174511b9f8a24e7a70d45a10e4c899bbda83ae0b"
)

func vectorKey() ed25519.PrivateKey {
	seed, _ := hex.DecodeString(vectorSeed)
	return ed25519.NewKeyFromSeed(seed)
}

func sign(key ed25519.PrivateKey, code string) (pub, sig string) {
	publicKey := key.Public().(ed25519.PublicKey)
	signature := ed25519.Sign(key, Frame(Message(code, publicKey)))
	return hex.EncodeToString(publicKey), hex.EncodeToString(signature)
}

func setup(t *testing.T) (*Service, *time.Time) {
	t.Helper()
	s, _ := store.Open("")
	clock := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	return New(s, func() time.Time { return clock }), &clock
}

func TestSharedVectorStaysStable(t *testing.T) {
	pub, sig := sign(vectorKey(), vectorCode)
	if pub != vectorPublic || sig != vectorSignature {
		t.Fatalf("vector changed: public=%s signature=%s", pub, sig)
	}
}

func TestAMachineLinksWithAValidProof(t *testing.T) {
	svc, _ := setup(t)
	code, _, err := svc.NewCode("user-1")
	if err != nil {
		t.Fatal(err)
	}
	pub, sig := sign(vectorKey(), code)
	device, err := svc.Link(code, pub, sig, " laptop ")
	if err != nil {
		t.Fatal(err)
	}
	if device.UserID != "user-1" || device.Label != "laptop" || device.PublicKey != pub {
		t.Fatalf("unexpected device %+v", device)
	}
}

func TestLowerCaseCodeIsAccepted(t *testing.T) {
	svc, _ := setup(t)
	code, _, _ := svc.NewCode("u")
	pub, sig := sign(vectorKey(), code)
	if _, err := svc.Link(lower(code), pub, sig, ""); err != nil {
		t.Fatal(err)
	}
}

func lower(s string) string {
	b := []byte(s)
	for i := range b {
		if b[i] >= 'A' && b[i] <= 'Z' {
			b[i] += 'a' - 'A'
		}
	}
	return string(b)
}

func TestProofForAnotherCodeOrKeyIsRefused(t *testing.T) {
	svc, _ := setup(t)
	code, _, _ := svc.NewCode("u")
	pub, sig := sign(vectorKey(), "ZZZZZZZZZZZZ") // signed another code
	if _, err := svc.Link(code, pub, sig, ""); !errors.Is(err, ErrInvalidProof) {
		t.Fatalf("other code: %v", err)
	}
	other := ed25519.NewKeyFromSeed(make([]byte, 32))
	_, otherSig := sign(other, code)
	if _, err := svc.Link(code, pub, otherSig, ""); !errors.Is(err, ErrInvalidProof) {
		t.Fatalf("other key: %v", err)
	}
}

func TestProofUnderAnotherDomainIsRefused(t *testing.T) {
	svc, _ := setup(t)
	code, _, _ := svc.NewCode("u")
	key := vectorKey()
	pk := key.Public().(ed25519.PublicKey)
	wrong := ed25519.Sign(key, append([]byte("sil/v1/event\x00"), Message(code, pk)...))
	if _, err := svc.Link(code, hex.EncodeToString(pk), hex.EncodeToString(wrong), ""); !errors.Is(err, ErrInvalidProof) {
		t.Fatalf("got %v", err)
	}
}

func TestCodeWorksOnceAndExpires(t *testing.T) {
	svc, clock := setup(t)
	code, _, _ := svc.NewCode("u")
	pub, sig := sign(vectorKey(), code)
	if _, err := svc.Link(code, pub, sig, ""); err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Link(code, pub, sig, ""); !errors.Is(err, ErrBadCode) {
		t.Fatalf("reuse: %v", err)
	}
	late, _, _ := svc.NewCode("u")
	*clock = clock.Add(CodeTTL + time.Second)
	pub2, sig2 := sign(ed25519.NewKeyFromSeed(make([]byte, 32)), late)
	if _, err := svc.Link(late, pub2, sig2, ""); !errors.Is(err, ErrBadCode) {
		t.Fatalf("expired: %v", err)
	}
}

func TestBadInputIsRefusedBeforeAnyWork(t *testing.T) {
	svc, _ := setup(t)
	pub, sig := sign(vectorKey(), vectorCode)
	for name, call := range map[string]func() error{
		"short code": func() error { _, e := svc.Link("ABC", pub, sig, ""); return e },
		"bad key":    func() error { _, e := svc.Link(vectorCode, "zz", sig, ""); return e },
		"upper hex":  func() error { _, e := svc.Link(vectorCode, upper(pub), sig, ""); return e },
		"bad sig":    func() error { _, e := svc.Link(vectorCode, pub, "00", ""); return e },
	} {
		if err := call(); !errors.Is(err, ErrInvalidInput) {
			t.Errorf("%s: got %v", name, err)
		}
	}
}

func upper(s string) string {
	b := []byte(s)
	for i := range b {
		if b[i] >= 'a' && b[i] <= 'f' {
			b[i] -= 'a' - 'A'
		}
	}
	return string(b)
}

func TestRevokedKeyCanBeLinkedAgainAndDeviceCountIsCapped(t *testing.T) {
	svc, _ := setup(t)
	code, _, _ := svc.NewCode("u")
	pub, sig := sign(vectorKey(), code)
	device, _ := svc.Link(code, pub, sig, "")
	if err := svc.Revoke("u", device.ID); err != nil {
		t.Fatal(err)
	}
	code, _, _ = svc.NewCode("u")
	pub, sig = sign(vectorKey(), code)
	if _, err := svc.Link(code, pub, sig, ""); err != nil {
		t.Fatalf("relink after revoke: %v", err)
	}
	for i := 0; i < MaxDevices; i++ {
		seed := make([]byte, 32)
		seed[0], seed[1] = byte(i+1), 9
		code, _, err := svc.NewCode("u")
		if err != nil {
			if !errors.Is(err, ErrTooMany) {
				t.Fatal(err)
			}
			return
		}
		pub, sig := sign(ed25519.NewKeyFromSeed(seed), code)
		if _, err := svc.Link(code, pub, sig, ""); err != nil {
			t.Fatal(err)
		}
	}
	t.Fatal("device cap never reached")
}
