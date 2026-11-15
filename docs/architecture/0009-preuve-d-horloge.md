# ADR 0009 — Clock proof for the ordering of claims

**Status**: implemented. `EventBody::Claim` carries `anchor_slot`; the order of claims is
`(anchor_slot, issued_at_ms, id)`; on live reception, a claim whose slot is outside the local window is
refused. Implementation choice: a tolerance of 2 slots above the local slot (`CLAIM_LAG_TOLERANCE_SLOTS`),
to allow for the lag of the local RPC; claims received outside live reception (catch-up) pass without a
bound, like the other historical events.

Accepted limit: the reception bound depends on the slot read by the node. Two nodes whose RPC differ by
more than the window can accept different sets of claims. The sort itself stays deterministic: it reads
only the slot signed in the event.

## Context

A `claim` wins an attempt according to the total order `(issued_at_ms, id)` (ADR 0008 and `sil-ledger`,
`state/attempt.rs`). However, `issued_at_ms` is declared by the author. A worker can backdate its claim and
win the race.

Current measure (not sufficient): `sil-node/src/ingest.rs` refuses, on live reception (gossip or API), an
event older than `MAX_LIVE_AGE_MS` (60 s) or newer than `MAX_SKEW_MS` (30 s). A backdating of up to 60 s
remains possible. The bound depends on each node's local clock, hence on its synchronization.

## Proposed decision

Anchor each `claim` to an external, verifiable clock: the recent Solana slot seen by the worker.

1. `EventBody::Claim` carries a field `anchor_slot: u64`, in addition to `fragment` and `attempt`.
2. The reducer orders the claims of an attempt by `(anchor_slot, issued_at_ms, id)`.
3. On live reception, a node checks that `anchor_slot` lies in a window `[slot_rpc - W, slot_rpc]`, with
   `W` of the order of 150 slots (about one minute). Outside the window, the event is refused.
4. A worker can therefore no longer backdate its claim by more than `W` slots, whatever its local clock.

## Consequences

- **Format**: change of the `Claim` type, hence of the canonical encoding. The vectors
  (`contracts/vectors/`) and the `sil-ledger` tests (about 20 constructors) must be updated.
- **RPC dependency**: every node that validates a claim must have a Solana RPC. A node without `[chain]`
  can no longer validate claims. Today, a worker without a chain already claims nothing.
- **Deployment**: incompatible change. All nodes (VPS, desktop machines, local nodes) must move to the new
  version at the same time. Existing ledgers are set aside (the `adr-0007` mechanism of the desktop app).
- **Limit**: two claims in the same slot (about 400 ms) remain ordered by `issued_at_ms`. The window is reduced
  to a few hundred milliseconds, not to zero.
- **RPC dependency**: if the RPC is down or lying, the window becomes false. Several RPCs or a cross-check are
  needed (a hypothesis to decide).

## Alternatives

- **Bounded local clock** (current state, 60 s): simple, but depends on local synchronization.
- **Order by reception at a witness**: not deterministic between nodes, rejected.
- **Causal proof** (reference to the last event seen): does not bound backdating without a window.
- **Light consensus clock**: disproportionate complexity for the testnet.

## Review fix: ledger rule on freshness

A reception bound alone is not sufficient: a backdated claim received through catch-up (sync) is not subject
to it, and the `(anchor_slot, …)` order would place it ahead of a fresh claim, redefining a winner already
accepted. A rule was added to the ledger (`sil-ledger`, `state/attempt.rs`): for an attempt, a claim is valid
only if `anchor_slot + CLAIM_WINDOW_SLOTS ≥ max(anchor_slot of the claims of this attempt)`. It depends on no
local clock. Accepted consequence: a fresh claim that arrives after an older claim can invalidate the older one,
because the most recent one sets the reference.
