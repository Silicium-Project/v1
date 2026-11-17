# ADR 0015 — One batch workload with operators

**Status**: proposed. No code change until this ADR is validated.

## Context

The network has two data workloads, both built on the same shape: a batch container (`SILBATCH`) split into
fragments, each fragment a range of items.

- `image/batch` (`contracts/workloads/image-batch.json`): every image is resized and re-encoded (PNG or JPEG) in
  WASM. Deterministic output, verified by sampling and re-execution.
- `embedding/e5-small` (`contracts/workloads/embedding-e5-small.json`, crate `sil-wl-embed`): every text chunk gets
  a 384-dimension vector from a pinned model. Output is not bit-exact across machines, so it is verified by sampling
  with a cosine threshold (`cos_min` 0.999).

Other data tasks (OCR, format conversion, dataset cleaning) would each add another workload, with its own
descriptor, its own verification code and its own website entry. The website lists workloads one by one
(`apps/web/src/i18n/*-site.ts`).

The job specification is part of the ledger (`crates/sil-ledger/src/spec.rs`, `job.rs`). A change to it changes
what nodes accept, so ADR 0014 applies: old specifications stay readable and verifiable.

## Decision

Replace the data workloads by one workload, `batch`. An operator names the processing; the operator decides the
verification.

- **Job** = `workload: batch`, `operator`, `params`, input container, fragment split, limits, terms.
- **Operator** = a WASM module (for deterministic operators) or a pinned model (for model operators), identified by
  its SHA-256. A node runs only operators it has in its built-in list. Adding an operator is a release.
- **Verification follows the operator**, not the workload:
  - `deterministic`: re-execution and output hash equality (resize, convert).
  - `tolerance`: sampling, each output compared to the re-computed one with a per-operator metric
    (embedding: cosine ≥ `cos_min`).
- **Operators at start**: `resize` (today's `image/batch`), `embed` (today's `embedding/e5-small`). OCR and format
  conversion are later operators, each one a separate release.
- **Existing jobs**: `image/batch` and `embedding/e5-small` keep their current specification and decoders. New jobs
  use `batch`. Nothing is migrated in place (ADR 0014: no rewrite of signed history).

## Open question: one website entry or one per operator

The protocol decision above does not depend on this. Two options for the website:

- **A. One entry, operator selector.** One form: input, operator, parameters. The website shows only the operators
  the node version supports. Less UI to maintain; a new operator appears without a new page.
- **B. One entry per operator.** A page per operator with its own help and defaults. Clearer for a newcomer, one
  more page per operator.

Recommendation: **A**, with help text per operator. The form is the same for every operator; only the parameters
change.

## Consequences

- Changes `sil-ledger` (job specification, new version), `sil-wl-embed` (becomes an operator), the image workload
  code, the descriptors, and the website.
- Verification code is split by metric (exact hash, cosine), not by product.
- The operator list is part of the release. A node refuses a job whose operator it does not know, with a clear
  error (same rule as an unknown ledger version).
- Data privacy is unchanged: only public data until a per-fragment encryption design exists.
- Native operators (ffmpeg, GPU ONNX) are out of this ADR. They need their own trust and sandbox decision.

## Plan (after validation)

1. Specification: `batch` version, operator field, verification metric per operator. Test vectors in
   `contracts/vectors`.
2. Port `resize` (exact hash) and `embed` (cosine) as operators; keep the old workloads' tests green.
3. Website: operator selector (option A), help per operator.
4. Release: old workload ids stay accepted for existing jobs; new submissions use `batch`.
