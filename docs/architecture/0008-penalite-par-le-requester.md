# ADR 0008: penalty signed by the requester

**Status:** accepted (request of 8 November 2026, "fix the limits").

## Context

ADR 0007 removed the committee and, with it, the penalties: no authority remained to sanction a verifier
that approves a false result. Without sanctions, a verifier never loses its reputation, and the
reputation gate (ADR 0006) only blocks unregistered keys. Sybil protection is weaker than in V0.

## Decision

The **requester** of the job can penalize an approver of a settled fragment:

- Instruction `Penalize(PenaltyArgs { fragment, verifier })`. The requester signs the transaction: no
  Ed25519 precompile, no committee.
- Rules: the transaction is signed by `terms.requester`; the verifier approved this fragment
  (`SettledFragment.approvers`); one penalty per (fragment, verifier); the verifier has a reputation
  account.
- Effect: score − 1,000, floor 0. Three penalties on three fragments remove a verifier from eligibility
  (5,000 → 2,000).

## Why the requester

It is the only party harmed by a false result: it pays for the job and receives the result. It therefore
has an interest in sanctioning. It gains nothing by sanctioning an honest verifier.

## Limits

- The requester can sanction wrongly. An honest verifier can therefore lose points. A challenge based on
  a proof of divergence (recomputation on chain) does not exist yet.
- A requester colluding with a worker can let a false result pass, without penalty.
- The penalty is manual: the requester must observe the fault (off-chain recomputation) and send the
  transaction.

## Consequences

- Adapter: `penalize_ix` (signed by the requester only).
- Program: instruction `Penalize`, reputation account written (its penalty is recorded in the escrow
  account: one entry per approver and fragment).
- The committee stays absent: only the requester authorizes a penalty.
