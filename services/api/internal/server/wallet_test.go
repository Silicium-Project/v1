package server

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/base64"
	"testing"

	"sil/api/internal/wallet"
)

// fakeWallet signs like a Solana wallet would: Ed25519 over the UTF-8 bytes of the message.
type fakeWallet struct {
	pub     ed25519.PublicKey
	priv    ed25519.PrivateKey
	address string
}

func newFakeWallet(t *testing.T) fakeWallet {
	t.Helper()
	pub, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	return fakeWallet{pub: pub, priv: priv, address: wallet.EncodeBase58(pub)}
}

func (f fakeWallet) sign(message string) string {
	return base64.StdEncoding.EncodeToString(ed25519.Sign(f.priv, []byte(message)))
}

func (e *env) walletLogin(t *testing.T, f fakeWallet) (int, map[string]any) {
	t.Helper()
	status, challenge := e.call(t, "POST", "/v1/auth/wallet/challenge", "", map[string]string{"address": f.address})
	if status != 201 {
		t.Fatalf("challenge: %d %v", status, challenge)
	}
	return e.call(t, "POST", "/v1/auth/wallet/login", "", map[string]string{
		"address":   f.address,
		"nonce":     challenge["nonce"].(string),
		"signature": f.sign(challenge["message"].(string)),
	})
}

func TestWalletLoginCreatesAnAccountOnceAndOpensASession(t *testing.T) {
	e := newEnv(t, true)
	f := newFakeWallet(t)

	status, out := e.walletLogin(t, f)
	if status != 200 {
		t.Fatalf("login: %d %v", status, out)
	}
	token := out["token"].(string)
	if status, profile := e.call(t, "GET", "/v1/account", token, nil); status != 200 || profile["address"] != f.address {
		t.Fatalf("profile: %d %v", status, profile)
	}

	// Second login: same account, not a new one.
	if status, _ := e.walletLogin(t, f); status != 200 {
		t.Fatalf("second login: %d", status)
	}
	if _, list := e.call(t, "GET", "/v1/account/export", token, nil); list["address"] != f.address {
		t.Fatalf("export: %v", list)
	}
}

func TestWalletLoginNeedsTheRightSignatureAndANonceOnce(t *testing.T) {
	e := newEnv(t, true)
	f := newFakeWallet(t)
	other := newFakeWallet(t)
	_, challenge := e.call(t, "POST", "/v1/auth/wallet/challenge", "", map[string]string{"address": f.address})
	nonce := challenge["nonce"].(string)
	message := challenge["message"].(string)

	// Signed by another wallet: refused, and the nonce is burned.
	status, _ := e.call(t, "POST", "/v1/auth/wallet/login", "", map[string]string{
		"address": f.address, "nonce": nonce, "signature": other.sign(message),
	})
	if status != 401 {
		t.Fatalf("forged signature: %d", status)
	}
	status, _ = e.call(t, "POST", "/v1/auth/wallet/login", "", map[string]string{
		"address": f.address, "nonce": nonce, "signature": f.sign(message),
	})
	if status != 401 {
		t.Fatalf("nonce reused after a failure: %d", status)
	}
}

func TestWalletSignatureCannotBeReplayedForAnotherAddress(t *testing.T) {
	e := newEnv(t, true)
	f := newFakeWallet(t)
	victim := newFakeWallet(t)
	_, challenge := e.call(t, "POST", "/v1/auth/wallet/challenge", "", map[string]string{"address": f.address})
	status, _ := e.call(t, "POST", "/v1/auth/wallet/login", "", map[string]string{
		"address":   victim.address,
		"nonce":     challenge["nonce"].(string),
		"signature": f.sign(challenge["message"].(string)),
	})
	if status != 401 {
		t.Fatalf("signature moved to another address: %d", status)
	}
}

func TestWalletAccountCannotLogInWithAPassword(t *testing.T) {
	e := newEnv(t, true)
	f := newFakeWallet(t)
	if status, _ := e.walletLogin(t, f); status != 200 {
		t.Fatalf("login: %d", status)
	}
	status, _ := e.call(t, "POST", "/v1/auth/login", "", map[string]string{"email": "", "password": "correct horse battery"})
	if status != 401 {
		t.Fatalf("password login: %d", status)
	}
}

func TestWalletAccountIsDeletedWithAFreshDeleteSignature(t *testing.T) {
	e := newEnv(t, true)
	f := newFakeWallet(t)
	status, out := e.walletLogin(t, f)
	if status != 200 {
		t.Fatalf("login: %d", status)
	}
	token := out["token"].(string)

	// A login signature must not delete the account.
	_, loginChallenge := e.call(t, "POST", "/v1/auth/wallet/challenge", "", map[string]string{"address": f.address})
	if status, _ := e.call(t, "DELETE", "/v1/account", token, map[string]string{
		"nonce": loginChallenge["nonce"].(string), "signature": f.sign(loginChallenge["message"].(string)),
	}); status != 401 {
		t.Fatalf("login signature deleted the account: %d", status)
	}

	_, challenge := e.call(t, "POST", "/v1/account/delete-challenge", token, nil)
	if status, _ := e.call(t, "DELETE", "/v1/account", token, map[string]string{
		"nonce": challenge["nonce"].(string), "signature": f.sign(challenge["message"].(string)),
	}); status != 204 {
		t.Fatalf("delete: %d", status)
	}
	if status, _ := e.walletLogin(t, f); status != 200 {
		t.Fatalf("a deleted wallet should get a fresh account on next login, got %d", status)
	}
}
