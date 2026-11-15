# ADR 0014 — Ledger migration between versions

**Status**: proposed. No migration code exists yet. Decision to be validated before implementation.

## Context

The ledger (`ledger/seg-*.log`) stores signed events as they are. A format change makes the old ledger unreadable
by the new version: `encodage non canonique` (non-canonical encoding) at start-up. This happened with 1.8 (anchored
claims, verdicts in a fixed binary frame). The nodes had to be reset, and their local history (pending earnings, local
reputation) was lost.

The version number `EVENT_VERSION` (u8, `crates/sil-ledger/src/event.rs`) exists, but it does not say how to read older
versions. No decoder for an old version is kept.

## Constraints

1. **Events are signed.** A migration cannot re-sign an event: it must keep the original bytes, or rewrite under an
   explicit authority. An event rewritten without its signature is no longer a proof.
2. **Rules change between versions.** An anchored claim (1.8) has no meaning under the rules of 1.7: freshness is judged
   at publication, not on re-reading.
3. **No automatic migration at start-up** (rule of `docs/technical/backup.md`, point 3).
4. **Verified backup before any migration** (existing protocol, `infra/vps/backup.sh`).

## Options

**A. Versioned decoders + offline migration tool.** The ledger carries a version header. The node keeps the decoders of
the versions still present. `silicium-migrate` reads an old ledger, verifies each signature with its decoder, and writes
a new ledger (same content, new format). Events whose rules have changed are copied as **closed history** (marked, never
replayed as active claims).

**B. Signed snapshot.** The node signs a state (reputation, finalized fragments, pending settlements) at a given instant
and keeps only the snapshot and the events after it. Simpler to evolve, but the historical proof is lost.

**C. Reset (current situation).** Acceptable for a test network, not for a real network.

## Proposed decision

Option **A** for the transition, then **B** as the long-term format once reputation and settlements are stable.

Rules:
- the ledger header carries `format_version`; a node refuses a ledger whose version it does not know, with a message
  pointing to `silicium-migrate`;
- `silicium-migrate` never writes over the original: it creates a `ledger.v<N>` directory next to it and prints the
  checksums before and after;
- every migration is preceded by a verified backup;
- the decoders of old versions are frozen (regression tests on real files, under `crates/sil-ledger/tests/fixtures/`).

## Consequences

- Nodes keep their identity (`node.seed`) and their signed history during an update.
- Cost: maintaining frozen decoders and per-version fixtures.
- The number of versions to migrate must stay bounded (the previous one, no more).
