package devices

import (
	"strings"
	"time"
	"unicode/utf8"
)

// Preview is what the machine shows before the owner confirms a link: which account the code
// belongs to. The account is masked; the full e-mail or wallet address is never returned.
type Preview struct {
	Account   string    `json:"account"`
	Kind      string    `json:"kind"` // "email" or "wallet"
	ExpiresAt time.Time `json:"expires_at"`
}

// Preview reads a usable link code without consuming it. A code carries 60 bits of randomness
// and lives 10 minutes, so an unauthenticated lookup cannot be enumerated.
func (s *Service) Preview(code string) (Preview, error) {
	code = strings.ToUpper(strings.TrimSpace(code))
	if !validCode(code) {
		return Preview{}, ErrInvalidInput
	}
	link, err := s.store.LinkByCode(hashCode(code), s.now())
	if err != nil {
		return Preview{}, ErrBadCode
	}
	user, err := s.store.UserByID(link.UserID)
	if err != nil {
		return Preview{}, ErrBadCode
	}
	preview := Preview{Kind: "wallet", Account: maskAddress(user.Address), ExpiresAt: link.ExpiresAt.UTC()}
	if user.Email != "" {
		preview.Kind, preview.Account = "email", maskEmail(user.Email)
	}
	return preview, nil
}

func validCode(code string) bool {
	if len(code) != codeLength {
		return false
	}
	for i := 0; i < len(code); i++ {
		if strings.IndexByte(codeAlphabet, code[i]) < 0 {
			return false
		}
	}
	return true
}

// maskEmail keeps the first letter and the domain: a***@example.com.
func maskEmail(email string) string {
	local, domain, found := strings.Cut(email, "@")
	if !found || local == "" {
		return "***"
	}
	first, _ := utf8.DecodeRuneInString(local)
	return string(first) + "***@" + domain
}

// maskAddress keeps the first and last four characters of a wallet address.
func maskAddress(address string) string {
	if len(address) <= 10 {
		return "***"
	}
	return address[:4] + "…" + address[len(address)-4:]
}
