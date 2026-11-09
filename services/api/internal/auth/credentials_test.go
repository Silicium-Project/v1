package auth

import (
	"errors"
	"strings"
	"testing"
	"time"

	"sil/api/internal/store"
)

// walletAccount creates an account with a wallet address and no e-mail or password.
func walletAccount(t *testing.T, svc *Service, id string) {
	t.Helper()
	if err := svc.store.CreateUser(store.User{ID: id, Address: "WALLET-" + id, CreatedAt: time.Now()}); err != nil {
		t.Fatal(err)
	}
}

func TestWalletAccountGetsEmailAndPassword(t *testing.T) {
	svc, _ := newService(t)
	walletAccount(t, svc, "w1")
	if err := svc.SetCredentials("w1", "Wallet@Example.com", "", goodPassword); err != nil {
		t.Fatal(err)
	}
	if _, _, err := svc.Login("wallet@example.com", goodPassword); err != nil {
		t.Fatalf("login with the new credentials refused: %v", err)
	}
}

func TestSettingAPasswordNeedsAValidOne(t *testing.T) {
	svc, _ := newService(t)
	walletAccount(t, svc, "w1")
	short := strings.Repeat("a", minPassword-1)
	long := strings.Repeat("a", maxPassword+1)
	for _, password := range []string{"", short, long} {
		if err := svc.SetCredentials("w1", "w@example.com", "", password); !errors.Is(err, ErrInvalidInput) {
			t.Fatalf("password of %d characters: want ErrInvalidInput, got %v", len(password), err)
		}
	}
}

func TestChangingCredentialsNeedsTheCurrentPassword(t *testing.T) {
	svc, _ := newService(t)
	walletAccount(t, svc, "w1")
	if err := svc.SetCredentials("w1", "w@example.com", "", goodPassword); err != nil {
		t.Fatal(err)
	}
	if err := svc.SetCredentials("w1", "other@example.com", "wrong password here", ""); !errors.Is(err, ErrWrongPassword) {
		t.Fatalf("wrong current password: want ErrWrongPassword, got %v", err)
	}
	if err := svc.SetCredentials("w1", "other@example.com", goodPassword, ""); err != nil {
		t.Fatalf("right current password refused: %v", err)
	}
}

func TestChangingOnlyTheEmailKeepsThePassword(t *testing.T) {
	svc, _ := newService(t)
	walletAccount(t, svc, "w1")
	_ = svc.SetCredentials("w1", "w@example.com", "", goodPassword)
	if err := svc.SetCredentials("w1", "new@example.com", goodPassword, ""); err != nil {
		t.Fatal(err)
	}
	if _, _, err := svc.Login("new@example.com", goodPassword); err != nil {
		t.Fatalf("old password no longer works after an e-mail change: %v", err)
	}
	if _, _, err := svc.Login("w@example.com", goodPassword); err == nil {
		t.Fatal("old e-mail still signs in")
	}
}

func TestChangingThePasswordReplacesTheOldOne(t *testing.T) {
	svc, _ := newService(t)
	walletAccount(t, svc, "w1")
	_ = svc.SetCredentials("w1", "w@example.com", "", goodPassword)
	const newPassword = "another long passphrase"
	if err := svc.SetCredentials("w1", "w@example.com", goodPassword, newPassword); err != nil {
		t.Fatal(err)
	}
	if _, _, err := svc.Login("w@example.com", goodPassword); err == nil {
		t.Fatal("old password still signs in")
	}
	if _, _, err := svc.Login("w@example.com", newPassword); err != nil {
		t.Fatalf("new password refused: %v", err)
	}
}

func TestEmailMustBelongToNobodyElse(t *testing.T) {
	svc, _ := newService(t)
	_, _ = svc.Register("taken@example.com", goodPassword)
	walletAccount(t, svc, "w1")
	if err := svc.SetCredentials("w1", "TAKEN@example.com", "", goodPassword); !errors.Is(err, ErrEmailTaken) {
		t.Fatalf("want ErrEmailTaken, got %v", err)
	}
}
