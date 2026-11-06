package config

import (
	"strings"
	"testing"
)

func TestCommitteeKeysAreCheckedLikeTheLedgerChecksThem(t *testing.T) {
	good := strings.Repeat("a", 64)
	other := strings.Repeat("b", 64)
	if problems := validateCommittee([]string{good, other}); len(problems) != 0 {
		t.Fatalf("valid committee refused: %v", problems)
	}
	cases := map[string][]string{
		"upper case":    {strings.Repeat("A", 64)},
		"short":         {"abcd"},
		"not hex":       {strings.Repeat("z", 64)},
		"duplicate":     {good, good},
		"eight members": {strings.Repeat("1", 64), strings.Repeat("2", 64), strings.Repeat("3", 64), strings.Repeat("4", 64), strings.Repeat("5", 64), strings.Repeat("6", 64), strings.Repeat("7", 64), strings.Repeat("8", 64)},
	}
	for name, keys := range cases {
		if problems := validateCommittee(keys); len(problems) == 0 {
			t.Errorf("%s accepted", name)
		}
	}
}
