# ADR 0002 — Finalization certificate

**Status:** proposed.

## Context
V0: `TaskFinalized` is accepted when a result exists, without checking the approval threshold.

## Decision
`TaskFinalized` carries at least k signed attestations from accepted verifiers. Every replica
re-validates the certificate; without a valid certificate the event is rejected. A rejection carries
a proof (signed divergent blocks), not a boolean.

## Consequences
A compromised coordinator cannot fabricate a finalization. The certificate is also the proof root
presented to the Solana program.

**Update (phase 2)**: the rejection carries an evidence reference (`evidence`), but the ledger does not
verify it (it is workload-specific). So that a single isolated verifier cannot block a result, V
rejections (V = `verifications_required`) are needed to reject, and V approvals plus V rejections give
the state `Disputed`. See protocol/ledger.md.
