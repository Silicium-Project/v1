# ADR 0006: on-chain verifier reputation, gate at settlement

**Status:** accepted (option A, 8 November 2026). **Updated by ADR 0007:** the committee penalties are removed; ADR 0008 reinstates them, signed by the requester.

## Context

ADR 0005 removes the coordinator vote: verifiers are the first `k` valid offers from authors distinct from
the claimant. The allowlist `SIL_API_COMMITTEE` (a fix for the "Sybil verifiers" risk) reintroduced a fixed
committee: only its members could verify. A job can then block for lack of members online (observed on 8
November, job `3e0763…`).

V0 did not bound selection with a list. Any node with the `verify` role could verify, and reputation played
a part in two places:

- in P2P, the publisher accepts offers;
- on-chain, `vote_fragment` requires `verify_reputation ≥ MIN_VERIFY_REPUTATION` (2,500) on the voter's
  `Worker` account (`chain/solana/programs/solana-layer/src/instructions/vote_fragment.rs:50`).

In V1, the reputation derived from the ledger (`sil-reputation`) does not weigh on selection. The reducer
must be deterministic over the ledger; the Solana state is not part of it, so it cannot read the on-chain
reputation without every node having the same view.

## Decision

The reputation check happens **at settlement**, on the chain:

1. One reputation account per verifier, PDA `[b"verifier", key]`, score `i64` starting at 5,000.
   `RegisterVerifier` creates it, paid by the signer.
2. `Settle` reads the score of each approver. Without an account, or below 2,500, the settlement is refused
   (`IneligibleVerifier`). Nothing is paid.
3. An accepted settlement adds 100 to each approver's score (cap 10,000).
4. `Penalize`, signed by the committee (`threshold` signatures, message `penalty_frame`), removes 1,000 from
   an approver of a settled fragment. Once per (fragment, verifier). Three faults remove a verifier from
   eligibility.

P2P selection stays open: the first `k` valid offers, with no allowlist. A non-eligible verifier can sign
offers, but the settlement will not pay it.

## Rules of ADR 0005 modified

- Rule 4 (verifiers): no allowlist. `SIL_API_COMMITTEE` no longer filters verification offers, neither in the
  reducer (`attempt.rs`, `finalize.rs`, `open.rs`) nor in the node (`verifier_loop.rs`). The list of settlement
  signers and sanctioning members remains.
- The reducer and the ledger do not change: no Solana data enters the P2P state.

## Consequences

- The committee remains the authority for settlement and penalties. Removing the committee from settlement is
  a separate decision: another settlement proof than the committee's signatures would then be needed.
- Entry cost: the rent of an account (about 0.001 SOL) and the transaction fees, paid once.
- A penalty assumes the committee observed the fault. This criterion is not automated yet.
- The node does not register its keys yet: `register_verifier_ix` exists in the adapter, but nothing calls it.
  Without registration, a verifier's settlements are refused.
- A job already open keeps its committee in its spec: this change does not unblock a stuck job.

## Discarded alternatives

- **B, snapshot in the ledger**: the offer carries the reputation read on-chain, and the reducer filters on it.
  No committee in selection, but the snapshot is declarative: a node can forge it without any account proof.
- **Ledger-derived reputation alone**: already in place (phase 1b), not used for selection. It is neither a
  public reputation nor one the payer can verify.

## Verification

- `sil-chain-core`: pure rules tested (`reputation`, `penalty`, `tests_reputation`, `tests_random`).
- `chain/program` and `chain/adapter`: compiled and tested outside the localnet. `cargo build-sbf` and
  `solana-test-validator` are not installed on the development machine.
