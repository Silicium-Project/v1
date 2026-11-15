# ADR 0010 — Withholding part of verifier rewards

**Status**: decided, not implemented. Changes the Solana program: all nodes must be updated.

## Context

One owner can control several keys. They can make these keys vote together as verifiers and validate their
own results. Today nothing costs a key that votes wrongly.

## Decision

A bond per verifier, taken from its earnings and not deposited in advance.

1. **Withheld share**: 50% of each vote's verifier reward is put in reserve on-chain, in the name of the key.
2. **Release**: the reserve of a vote is released at the first deadline reached among:
   - 20 settlements after the key's last vote;
   - 24 hours after the key's last vote.
3. **Penalty**: if a vote of the key is judged false (disagreement with the accepted result, ADR 0008), the
   key's reserve is confiscated, in whole or in part. The exact scale remains to be set.
4. **Selection**: the verifiers of a fragment are drawn unpredictably for the author of the result. The draw
   uses a value that nobody chooses alone (a Solana slot hash).
5. **Number of verifiers**: to be set. No value is chosen yet.

## Consequences

- An owner who multiplies keys must lock 50% of their verification earnings during the release period, and
  risks losing them.
- A new key has no bond to deposit: it accumulates one. The entry cost is low, but the first period of
  earnings is locked.
- **Format**: a new reserve account per key in the Solana program, new release and confiscation instructions.
  Incompatible deployment: all nodes change at the same time.
- An inactive key recovers its reserve at the 24-hour deadline.

## Open points

- Definition of a "settlement" counted for the 20 (all settlements of the program, or only those of the
  verifier).
- Confiscation scale (total or proportional).
- Number of verifiers per fragment.

## Review fix: per-job release and confiscation

- **Release**: the counter counts only the verifier's **own** votes, since its oldest pending reserve
  (`pending_since`). Anyone can no longer advance it by settling other jobs. The 24-hour deadline after the
  last vote stays in place.
- **Per job**: each `SettledFragment` carries `held` (the reserve of each approver on this fragment). A penalty
  consumes only what this escrow withheld for the verifier; a release (`ReleaseReserve`, with the escrows as
  accounts) only takes what these escrows hold. No more cross-job confiscation.
- The global statistics account is removed.
