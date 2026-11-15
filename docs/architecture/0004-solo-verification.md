> **Superseded by ADR 0007:** the `SoloAccept` vote and the solo mode are removed.

# ADR 0004: solo verification (a lone node verifies its own computation)

## Context

A result must be verified by a verifier **distinct** from the claimant (`verdicts.rs`:
`the_claimant_cannot_be_its_own_verifier`, and `security.rs`: the claimant cannot inflate the
certificate with its own approval). On a network of a single node, nothing can therefore finalize a
computation. The need: be able to test and run the network alone, without weakening the normal case.

## Decision

A new committee vote, `SoloAccept { fragment, attempt, offer }`, accepts **the claimant's own
verification offer**. It is valid only if the offer is a `VerifyOffer` of the attempt **issued by the
claimant**. The result carries the flag `solo` (`VerifierState.solo`).

Path, for an attempt published at `t0`:

1. the claimant publishes its `Submit`;
2. the claimant waits `solo_wait_ms`; if no other verifier has offered itself
   (`open_verifications` empty, excluding the claimant), it issues its own `VerifyOffer`;
3. at `t0 + solo_wait_ms`, with no other offer and no accepted verifier, the committee votes
   `SoloAccept` on that offer;
4. the claimant verifies its computation and publishes its `Attest`, like an ordinary verifier.

The normal case does not change: `VerifierAccept` still accepts only offers from authors other than
the claimant. `SoloAccept` accepts only offers from the claimant.

## Guardrails

- **Disabled by default** (`[testnet] solo_verify = false`).
- **Refused with `[chain]`**: a result that pays money must be verified by a third party.
- **Visible**: `solo: true` in the job state. An interface can show "verified by the only node", and
  the reputation report can take it into account.
- **Decided by the committee**: the claimant cannot accept itself. A `SoloAccept` issued by a
  non-member is refused on insertion (`job.rs`).
- **Only if nobody else offered**: while another verifier exists, the normal case applies.

## Consequences

- A **solo** result is not a controlled result: the claimant judges its own computation. On an open
  network, a dishonest worker can get a false result accepted if it is alone or if the other verifiers
  stay silent during `solo_wait_ms`. This is acceptable only in testing.
- The committee alone decides the window. A cheating committee can accept a solo result early: same
  trust as for the other committee votes.
- The format of existing events does not change: `SoloAccept` is an added variant.

## Discarded alternatives

- **Remove the "no self-verification" rule**: the claimant would always approve its own computation,
  with no trace. Rejected.
- **Check "alone on the network" through presence (gossip)**: non-deterministic, not verifiable by
  other nodes. Rejected: the decision stays in the signed ledger.
- **A node without verification (accept without checking)**: worse; the `solo` flag makes it visible.

## Verification

- `sil-ledger`: a `SoloAccept` on its own offer accepts the claimant (`solo: true`); a `SoloAccept` on
  another author's offer is ignored.
- `sil-coordinator`: no vote before the delay; vote after the delay; no solo if another verifier offered
  itself; no solo without `with_solo`.
- `sil-node`: a lone node finalizes a job (`a_lone_node_verifies_its_own_result_and_finalizes`).
