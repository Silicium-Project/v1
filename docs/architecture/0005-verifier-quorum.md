# ADR 0005: verifier quorum, without a coordinator committee

## Context

Today a job is decided by a fixed committee of three coordinators (ADR 0001). They vote on claims
(`Accept`), on verifiers (`VerifierAccept`) and on finalization (`Finalize`). Three limits follow:

- **Liveness**: without two coordinators online, no fragment is assigned (observed on the devnet: job
  `Active` with no attempt).
- **Centralization**: the three keys are chosen by the operator (`SIL_API_COMMITTEE`).
- **Local reputation**: each node computes the reputation of workers and verifiers from its own
  observations. Two nodes can rank offers differently and disagree on the selected verifier.

The need: a network where one worker and one verifier are enough to move a computation forward, with no
voting body, and where all nodes decide the same way from the same ledger.

## Decision

No more coordinator votes. Every decision is a **deterministic rule over the ledger state**. All nodes
holding the same ledger obtain the same result. A node no longer "votes": it publishes a signed event
(claim, verification offer, verdict), and the other events are read, not voted on.

### Rules

1. **Total order of events.** Sorted by `(issued_at_ms, event identifier)`. An event dated more than
   `MAX_SKEW_MS` in the future relative to the receiving node is rejected (see open questions).
2. **Claim.** For a (fragment, attempt), the winner is the first valid `Claim` in the total order. A
   `Claim` is valid if the claimant is allowed to claim (healthy escrow, when the chain is active) and if
   the fragment has no attempt in progress.
3. **Lease.** An attempt with no `Submit` or `Renew` within its lease (`lease_ms`) expires. The fragment
   returns to `Open` and a new attempt can be claimed.
4. **Verifiers.** The job spec sets `k` (`verifications_required`). The first `k` valid verification
   offers, from authors distinct from the claimant, are accepted in total order. A verifier with a low
   verifier reputation may be ranked after the other offers, as today, provided that the ranking is
   computed from the ledger (rule 5).
5. **Reputation derived from the ledger.** The score of an author (worker and verifier) is computed from
   events alone: attempts won or lost, verdicts correct or false. There is no longer any local
   observation. This follows the V0 principle (`author_records`).
6. **Finalization.** A fragment is finalized when `k` `Attest` events approve the winning attempt, with no
   valid rejection. Any node can publish the `Finalize`, with the `k` attestations as certificate
   (ADR 0002). The `Finalize` no longer needs to be signed by a coordinator.
7. **No self-validation.** The claimant is never one of its own verifiers.

## Consequences

- **Consensus change.** All nodes must switch to the new rule together: an old node and a new node would
  decide differently. The deployment is coordinated (VPS, sims, installers), as for the verifier ranking.
- **Coordinator role removed** from the protocol. The voting code stays compilable during the transition,
  but is no longer called.
- **Security**: with `k = 1`, a single verifier decides (acceptable on the devnet, not in production). With
  `k` at 2 or more, two independent verifiers are required, so at least three distinct machines (worker,
  two verifiers).
- **Existing ledger**: the devnet can restart from a new genesis. No migration of past decisions.

## Open questions

- **Clock drift.** The total order depends on `issued_at_ms`, hence on the peers' clocks. A peer that
  advances its clock can take a claim. Option: a `MAX_SKEW_MS` tolerance (30 s) and rejection of events
  outside the window. To decide before phase 2.
  *Derived reputation (phase 1b)*: the judgment time is the clock of the job's committee
  (`derived_reputation/clock.rs`, the threshold-th largest head date), not the node's clock. A single
  member neither advances nor freezes a lease. Consensus (claims, leases) does not read this clock yet:
  the lease rule remains to be written in phase 2.
- **Value of `k`** by default (1 in tests, 2 or more in production) and per workload.
- **Rejection by proof.** A verifier that rejects must publish a proof of divergence (signed divergent
  blocks, ADR 0004). The format remains to be defined.
- **Sybil.** Without a committee, a disposable key can try to become a verifier. Derived reputation and
  the probation mode (one turn in three) limit the risk without eliminating it.

## Plan

1. **Reputation derived from the ledger**: a pure function over the ledger state, tested so that two nodes
   with the same ledger get the same score. No consensus change.
   *Done*: `sil_reputation::from_observations` (phase 1a); `sil-node/src/derived_reputation.rs` reads the
   ledger and exposes it in the worker report (`reputation_derived`, phase 1b). Decisions (verifier ranking,
   committee) do not read it yet.
2. **Claim and lease without a vote**: the reducer keeps the first valid claim in total order. Tests with
   reordered events and a late claim.
   *Done* (`sil-ledger/src/state/attempt.rs`, tests `tests/claims.rs`):
   - a claim is valid if its author is not a former claimant of the fragment and, for attempt `k` above 1,
     if attempt `k-1` is released, rejected, or has an expired lease **at the issue time of the claim**;
   - the lease is `claim.issued + lease_ms`, extended by each renewal issued before the deadline; a
     `Submit` issued after the deadline has no effect;
   - the date of a received event is bounded: `MAX_SKEW_MS` = 30 s beyond the node's clock
     (`sil-node/src/ingest.rs`). This is the only place where the node reads its clock.
   - Gap fixed in phase 3 (test `a_backdated_claim_cannot_undo_a_finalization`): a claim dated before the one
     that succeeded, received after finalization, no longer undoes the certification.
3. **Verifiers and quorum `k`**: first `k` offers accepted, finalization on `k` attestations. Tests: one
   worker and one verifier (`k = 1`), two verifiers (`k = 2`), a dishonest verifier in the minority.
4. **Removal of the vote** from the node loops; local end-to-end test with three nodes and no coordinator.
   *Risks addressed*:
   - **Allowlist** (replaced by ADR 0006): the first version accepted only members of the job's committee
     (`SIL_API_COMMITTEE`). That committee blocked jobs when no members were online. Now any valid offer can
     be accepted; on-chain reputation is checked at settlement.
   - **Finality**: `Finalize` designates the offers of the approving verifiers (`verifiers`). Its validity
     reads only those events: an older offer received late undoes nothing.
   - **Solo**: disabled by default (`solo_wait_ms` absent). The only vote remaining, with the settlement
     signatures, is phase 5.
   *Done*: `coordinator_loop` no longer votes; any node publishes `Finalize` (`sil_coordinator::finalization`).
   Test `tests/no_coordinator.rs` (three nodes, no `Accept`/`VerifierAccept`). Remaining: the `SoloAccept` vote
   (ADR 0004) and the committee's settlement signatures (phase 5).
5. **Coordinated update** of the VPS, the sims and the installers, then a test on the devnet.
