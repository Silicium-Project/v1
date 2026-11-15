# ADR 0012 — Quorum vote, minority penalty and sharing of the bond

**Status**: implemented on the chain side (`sil-chain-core`, program, adapter). The proof is the verdict already
signed by each verifier (`Domain::Attestation`), rebuilt on-chain in a fixed binary format (`attestation_frame`,
identical to `sil_attest::verdict_message`). Contrary = approving another result, or rejecting the settled result
with its proof. The node has nothing more to produce.

Sending: the node detects the contrary verdicts of the selected verifiers (`penalty_claims`, on the finalized
attempt), queues them (`PenaltyBox`, sil-settlement) once the settlement is confirmed, and sends them through
`Chain::penalize`. Only a node that pays the fees sends. The queue is not persisted: it is rebuilt from the ledger
on every round. Tolerance verdicts (embedding) are covered: they have their own frame (`tolerance_frame`, domain
`sil/v1/attestation-tolerance`), and only those whose model and runtime match the job's count. This complements
ADR 0010 and replaces the penalty signed by the requester (ADR 0008). Today the code confiscates the bond on the
requester's signature alone and pays it to the requester.

## Flow

1. The requester submits a task on the website. It opens the escrow and deposits the funds.
2. Workers pick up the fragments, compute, and publish their results.
3. A quorum of `k` verifiers checks each fragment and votes. Votes are signed events of the ledger; the quorum
   itself stays off-chain.
4. The accepted result is the majority's. The majority's approvers sign the settlement, which is on-chain.
5. A verifier that voted against the majority is considered malicious. It is penalized.
6. If no verifier was wrong, nobody touches the bond: each verifier recovers it at its deadline (ADR 0010: 20
   settlements or 24 h after its last vote).

## Proposed decision

### 1. Minority penalty

A verifier that voted against the majority on a fragment loses reputation (ADR 0008) and the bond of that
fragment: the part of its reserve equal to 50% of that fragment's verifier reward, capped at its current reserve
(ADR 0010).

### 2. Sharing of the confiscated bond

- **70%** to the requester.
- **30%** shared equally among the quorum members who voted with the majority.
- Without a malicious verifier, nothing is confiscated.

### 3. Proof

The quorum being off-chain, the proof comes from the penalty transaction:

- **Majority**: the list of settled approvers is already on-chain. It gives the beneficiaries of the 30%, with
  nothing added.
- **Minority**: the transaction attaches the signature of the offender's vote on a result different from the
  settled result, for the same job, fragment and attempt. The program verifies it with the Ed25519 precompile,
  as for a settlement. The offender signed that vote itself: nobody can forge it on its behalf.
- **Who decides**: the quorum. The penalty follows from the signature of the vote contrary to the majority, not
  from a decision of the requester. The requester has no penalty power. Anyone can send the transaction, but the
  program applies the penalty only if the proof of contrary vote is valid. A single submission per (fragment,
  verifier).

### 4. Size of the quorum `k`

`k` is always odd. A strict majority therefore always exists: it is `(k+1)/2` votes, and there is never a tie.

`k` depends on the value of the task and on the number of available verifiers. Default rule:
`k = largest odd number ≤ min(max(k_value, k_min), k_available, 15)`, that is, the highest requirement, capped
by the available verifiers, and rounded down to an odd number. The cap is 15: 16 is even. Variant to decide:
weighting by reputation.

## Consequences

- **No sybil proof needed for the majority**: only verifiers that signed the settlement receive the 30%. A
  requester cannot fabricate a third party's vote.
- **Format**: a new signed vote message (its own domain, to be defined) and a new form of `Penalize` (signature of
  the offending vote attached). The escrow account does not change.
- **End of the requester's unilateral power**: it can no longer confiscate without a signed contrary vote.
- **The requester is penalized in nothing** if there is no offender: the bond goes back to the verifier.
- Incompatible deployment, as in ADR 0010.

## Open points

- Majority: `(k+1)/2` identical votes (settled, k odd). No more ties to handle.
- A verifier that does not vote at all is not targeted.
- A signed vote that was not published in the ledger before settlement: under which condition it counts as
  "against".
- Signed vote message and signing domain.
- Value of `k_min` and the function `k_value`.
- Window for submitting the proof: at the latest before the release of the reserve (ADR 0010).
