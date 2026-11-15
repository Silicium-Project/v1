# ADR 0003 — Scoped reputation and anti-Sybil

**Status:** proposed; open question (light stake or probationary period).

## Context
V0: reputation counts tasks from any publisher. An attacker publishes and verifies its own tasks to
inflate its score, and a new key resets the score to zero at no cost.

## Decision
Reputation is computed per committee/requester, with decay, weighted by the value independently
verified. Self-publication and self-verification are excluded from the computation. New keys go
through a probationary period with a bounded load.

## To decide
Light Solana stake versus a probationary period alone.

**Update (phase 4)**: implemented in `sil-reputation` (trust scopes, self-evaluation ignored, bounded
weight, 7-day half-life, one-task probation, bounded memory). The entry cost (stake) remains open: a
discarded key regains probation. See protocol/coordination.md.
