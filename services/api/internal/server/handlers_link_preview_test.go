package server

import "testing"

// The desktop application asks for the owner of a code before linking. No session is needed,
// the answer names the account only in masked form, and the code stays usable afterwards.
func TestLinkCodeCanBePreviewedWithoutASession(t *testing.T) {
	e := newEnv(t, true)
	token := e.login(t, "alice@example.com")
	_, out := e.call(t, "POST", "/v1/devices/link-codes", token, nil)
	code := out["code"].(string)

	status, preview := e.call(t, "GET", "/v1/devices/link-codes/"+code, "", nil)
	if status != 200 || preview["kind"] != "email" || preview["account"] != "a***@example.com" {
		t.Fatalf("preview: %d %v", status, preview)
	}
	if _, leaked := preview["email"]; leaked {
		t.Fatalf("the full e-mail must not be returned: %v", preview)
	}
	if status, _ := e.call(t, "GET", "/v1/devices/link-codes/BCDFGHJKMNPQ", "", nil); status != 400 {
		t.Fatalf("unknown code: %d", status)
	}
}
