// Package wallet verifies Solana wallet signatures: base58 addresses, single-use challenges
// and Ed25519 checks over the exact message text the user saw.
package wallet

import (
	"errors"
	"math/big"
)

const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

var errBase58 = errors.New("invalid base58")

// DecodeBase58 decodes a Solana address (base58) to its raw bytes.
func DecodeBase58(text string) ([]byte, error) {
	if text == "" {
		return nil, errBase58
	}
	n := new(big.Int)
	base := big.NewInt(58)
	for _, r := range text {
		index := indexOf(r)
		if index < 0 {
			return nil, errBase58
		}
		n.Mul(n, base)
		n.Add(n, big.NewInt(int64(index)))
	}
	raw := n.Bytes()
	// Each leading '1' stands for one zero byte.
	zeros := 0
	for zeros < len(text) && text[zeros] == '1' {
		zeros++
	}
	out := make([]byte, zeros+len(raw))
	copy(out[zeros:], raw)
	return out, nil
}

// EncodeBase58 encodes raw bytes as base58 (the canonical text form of an address).
func EncodeBase58(raw []byte) string {
	zeros := 0
	for zeros < len(raw) && raw[zeros] == 0 {
		zeros++
	}
	n := new(big.Int).SetBytes(raw)
	base := big.NewInt(58)
	mod := new(big.Int)
	var digits []byte
	for n.Sign() > 0 {
		n.DivMod(n, base, mod)
		digits = append(digits, alphabet[mod.Int64()])
	}
	for i := 0; i < zeros; i++ {
		digits = append(digits, alphabet[0])
	}
	// Digits were produced least significant first.
	for i, j := 0, len(digits)-1; i < j; i, j = i+1, j-1 {
		digits[i], digits[j] = digits[j], digits[i]
	}
	return string(digits)
}

func indexOf(r rune) int {
	for i, c := range alphabet {
		if c == r {
			return i
		}
	}
	return -1
}
