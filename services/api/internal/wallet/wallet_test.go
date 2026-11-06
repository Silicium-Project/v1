package wallet

import (
	"crypto/ed25519"
	"crypto/rand"
	"errors"
	"strings"
	"testing"
	"time"
)

func TestBase58KnownVectors(t *testing.T) {
	raw, err := DecodeBase58("JxF12TrwUP45BMd")
	if err != nil || string(raw) != "Hello World" {
		t.Fatalf("decode: %q %v", raw, err)
	}
	if got := EncodeBase58([]byte("Hello World")); got != "JxF12TrwUP45BMd" {
		t.Fatalf("encode: %q", got)
	}
	// Leading zero bytes are kept as leading '1' characters.
	if got := EncodeBase58([]byte{0, 0, 1}); got != "112" {
		t.Fatalf("leading zeros: %q", got)
	}
	back, _ := DecodeBase58("112")
	if len(back) != 3 || back[0] != 0 || back[1] != 0 || back[2] != 1 {
		t.Fatalf("leading zeros decode: %v", back)
	}
}

func TestBase58RejectsForeignCharacters(t *testing.T) {
	for _, bad := range []string{"", "0OIl", "abc!", "héllo"} {
		if _, err := DecodeBase58(bad); err == nil {
			t.Errorf("%q accepted", bad)
		}
	}
}

func newKey(t *testing.T) (ed25519.PublicKey, ed25519.PrivateKey, [32]byte) {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	var address [32]byte
	copy(address[:], pub)
	return pub, priv, address
}

func TestChallengeIsSingleUseAndBound(t *testing.T) {
	clock := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	c := NewChallenges(5*time.Minute, 100, func() time.Time { return clock })
	_, _, address := newKey(t)
	var other [32]byte
	other[0] = 1

	nonce, message, _, err := c.Issue(address, PurposeLogin)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(message, nonce) || !strings.Contains(message, "login") {
		t.Fatalf("message does not bind nonce and purpose: %q", message)
	}
	if _, err := c.Consume(nonce, other, PurposeLogin); !errors.Is(err, ErrChallenge) {
		t.Fatalf("other address: %v", err)
	}
	// The failed attempt above already burned the nonce.
	if _, err := c.Consume(nonce, address, PurposeLogin); !errors.Is(err, ErrChallenge) {
		t.Fatalf("reuse after failure: %v", err)
	}

	nonce, _, _, _ = c.Issue(address, PurposeLogin)
	if _, err := c.Consume(nonce, address, PurposeDelete); !errors.Is(err, ErrChallenge) {
		t.Fatalf("login nonce used for deletion: %v", err)
	}
}

func TestChallengeExpires(t *testing.T) {
	clock := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	c := NewChallenges(time.Minute, 100, func() time.Time { return clock })
	_, _, address := newKey(t)
	nonce, _, _, _ := c.Issue(address, PurposeLogin)
	clock = clock.Add(time.Minute)
	if _, err := c.Consume(nonce, address, PurposeLogin); !errors.Is(err, ErrChallenge) {
		t.Fatalf("expired nonce accepted: %v", err)
	}
}

func TestOpenChallengesAreBounded(t *testing.T) {
	c := NewChallenges(time.Hour, 2, nil)
	_, _, address := newKey(t)
	for i := 0; i < 2; i++ {
		if _, _, _, err := c.Issue(address, PurposeLogin); err != nil {
			t.Fatal(err)
		}
	}
	if _, _, _, err := c.Issue(address, PurposeLogin); !errors.Is(err, ErrFull) {
		t.Fatalf("third challenge: %v", err)
	}
}

func TestVerifyAcceptsTheWalletSignatureOnly(t *testing.T) {
	_, priv, address := newKey(t)
	c := NewChallenges(time.Hour, 10, nil)
	nonce, message, _, _ := c.Issue(address, PurposeLogin)
	signature := ed25519.Sign(priv, []byte(message))
	if !Verify(address, message, signature) {
		t.Fatal("valid signature refused")
	}
	if Verify(address, message+" ", signature) {
		t.Fatal("altered message accepted")
	}
	if Verify(address, message, signature[:63]) {
		t.Fatal("short signature accepted")
	}
	_ = nonce
}
