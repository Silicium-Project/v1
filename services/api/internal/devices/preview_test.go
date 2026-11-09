package devices

import (
	"errors"
	"testing"
	"time"

	"sil/api/internal/store"
)

func TestPreviewNamesTheAccountWithoutConsumingTheCode(t *testing.T) {
	svc, _ := setup(t)
	if err := svc.store.CreateUser(store.User{ID: "user-1", Email: "alice@example.com", CreatedAt: time.Now()}); err != nil {
		t.Fatal(err)
	}
	code, _, err := svc.NewCode("user-1")
	if err != nil {
		t.Fatal(err)
	}
	preview, err := svc.Preview(lower(code))
	if err != nil {
		t.Fatal(err)
	}
	if preview.Kind != "email" || preview.Account != "a***@example.com" {
		t.Fatalf("unexpected preview %+v", preview)
	}
	pub, sig := sign(vectorKey(), code)
	if _, err := svc.Link(code, pub, sig, "pc"); err != nil {
		t.Fatalf("preview used up the code: %v", err)
	}
}

func TestPreviewOfAWalletAccountMasksTheAddress(t *testing.T) {
	svc, _ := setup(t)
	address := "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"
	if err := svc.store.CreateUser(store.User{ID: "user-2", Address: address, CreatedAt: time.Now()}); err != nil {
		t.Fatal(err)
	}
	code, _, err := svc.NewCode("user-2")
	if err != nil {
		t.Fatal(err)
	}
	preview, err := svc.Preview(code)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Kind != "wallet" || preview.Account != "9xQe…VFin" {
		t.Fatalf("unexpected preview %+v", preview)
	}
}

func TestPreviewRefusesUnknownExpiredAndMalformedCodes(t *testing.T) {
	svc, now := setup(t)
	if err := svc.store.CreateUser(store.User{ID: "user-1", Email: "alice@example.com", CreatedAt: time.Now()}); err != nil {
		t.Fatal(err)
	}
	if _, err := svc.Preview("ABCDEFGHJKLM"); !errors.Is(err, ErrBadCode) {
		t.Fatalf("unknown code: %v", err)
	}
	for _, bad := range []string{"", "SHORT", "ABCDEFGHJKL0", "ABCDEFGHJKLI"} {
		if _, err := svc.Preview(bad); !errors.Is(err, ErrInvalidInput) {
			t.Fatalf("malformed %q: %v", bad, err)
		}
	}
	code, _, err := svc.NewCode("user-1")
	if err != nil {
		t.Fatal(err)
	}
	*now = now.Add(CodeTTL)
	if _, err := svc.Preview(code); !errors.Is(err, ErrBadCode) {
		t.Fatalf("expired code: %v", err)
	}
}

func TestMaskingNeverKeepsTheWholeAccount(t *testing.T) {
	if got := maskEmail("bob@mail.example.org"); got != "b***@mail.example.org" {
		t.Fatalf("email: %s", got)
	}
	if got := maskEmail("no-at-sign"); got != "***" {
		t.Fatalf("email without @: %s", got)
	}
	if got := maskAddress("short"); got != "***" {
		t.Fatalf("short address: %s", got)
	}
}
