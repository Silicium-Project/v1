# ADR 0001 — Per-job coordination committee

**Status:** superseded by ADR 0007 (committee removed).

## Context
V0: a single publisher per task (single point of failure; finalization without proof of quorum).

## Decision
The requester signs a `JobSpec` listing n coordinators and a threshold m **strictly greater than
n/2**. A decision (claim accepted, verifier accepted, finalization) exists when m distinct members
have emitted the same vote: each vote is an ordinary event on the member's own chain (no aggregated
signature). The strict majority makes two contradictory decisions impossible. No global BFT.

**Update (phase 2)**: the epoch rotation planned at first is not implemented. Liveness requires m
reachable members; under partition, waiting is safe. Rotation remains to be studied with phase 4
(coordinator).

## Alternatives
- Single publisher with manual recovery: simple, but a single point of failure.
- Global consensus (BFT): heavy, unnecessary for independent jobs.

## Consequences
More messages per job. Under partition, waiting is safe (never a forced finalization).
The values of (n, m) must be set by measurement in phase 4.
