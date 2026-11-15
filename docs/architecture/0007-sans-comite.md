# ADR 0007: removal of the committee

**Status:** accepted (request of 8 November 2026, after ADR 0005 and 0006).

## Context

ADR 0001 set a fixed committee of coordinators per job (`threshold` over `n`). ADR 0005 removed the
votes, but the committee remained: in the job spec, in the `SoloAccept` votes of solo mode (ADR 0004),
in the settlement signatures read by the chain, and in the committee penalty (ADR 0006). A job can
block for lack of committee members online (job `3e0763…`, 8 November).

V0 has no committee. Verifiers commit themselves, with their own key and reputation.

## Decision

1. **No committee and no threshold.** `JobSpec` loses `committee` and `threshold`. The signed text of
   the job loses the `Committee:` and `Threshold:` lines.
2. **No solo mode.** `SoloAccept` is removed. A claimant never verifies its own result. A job with
   `k` verifications therefore needs at least `k + 1` verifier nodes.
3. **Open verification** (ADR 0005, rule 4): the first `k` valid offers, from distinct authors, excluding
   the claimant.
4. **Finalization**: a certificate of `k` approvals from distinct verifiers (unchanged). Any node can
   publish it.
5. **Settlement authorization = approver signatures.** Each approver signs, with `Domain::Settlement`,
   the message `job ‖ fragment ‖ attempt ‖ result ‖ claimant`. It does so when it approves, and the
   signature is carried by its `Attestation`. The message does not contain the list of approvers: it is
   known only at finalization. The chain requires that each approver has signed, that each has a
   reputation score of at least 2,500, that there are `k` of them, all distinct and different from the
   claimant.
6. **Penalties**: removed by ADR 0007, reinstated for the requester (ADR 0008). The score increases only
   at each approved settlement (+100). **Limit corrected by ADR 0008:** without penalties, a verifier never
   loses its reputation. The gate only protects against unregistered keys. A proof-based challenge
   mechanism remains to be built.
7. **Clock of the derivation** (`sil-node`, derived reputation): `as_of` = the most recent event date of
   the job. No committee is needed to set it. This reputation stays displayed without weighing on
   decisions.
8. **Protocol**: new marker `adr-0007`. Earlier ledgers are set aside at start-up (mechanism of
   `node_process.rs`).

## Consequences

- Old jobs (spec with a committee) cannot be resumed: they are abandoned.
- The API loses `SIL_API_COMMITTEE` and `GET /v1/network/committee`.
- The VPS must be redeployed, and the installer republished (not done here: a decision to take).
- `sil-coordinator` is removed. Its only useful function, building `Finalize`, moves into `sil-node`.
- The node's `coordinator` role no longer designates a committee. It is removed from the configuration.

## Discarded alternatives

- **Keep a committee for settlement only**: the request was to remove it entirely.
- **On-chain vote per verifier (V0)**: same effect, more transactions per verifier, and a reward per vote
  to define. Deferred.
- **Penalty signed by peers**: no clear authority; to be designed together with the challenge mechanism.
