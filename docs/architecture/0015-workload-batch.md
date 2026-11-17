# ADR 0015 — One batch workload with operators

**Status**: accepted (validated by the maintainer). Implementation follows the plan below.

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
- **Parameter schema per operator**: each operator declares the names, types, bounds and defaults of its `params`.
  The schema is part of the operator's release. The node validates `params` against it before claiming any fragment,
  and refuses a job whose params do not match (same error path as an unknown operator).
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

## Website

The protocol does not depend on the website. The website has three parts:

- **One page per operator** (the entry point a user finds and understands): what the operator does, an example,
  the cost, and a link to submit. Discovery is easier this way.
- **One submission component, shared by all pages.** It takes the operator and renders the form from that
  operator's parameter schema. The operator is fixed by the page that opened it. No per-operator form code.
- **A catalogue** listing the operator pages, grouped by category (image, text, data), so a user who does not know
  the operator name still finds it.

The per-operator pages are content only (text, example, translations fr/en). They cannot diverge from the protocol:
they show only the operators the connected network version supports.

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
3. Website: shared submission component driven by the parameter schema; one page per operator; catalogue by
   category.
4. Release: old workload ids stay accepted for existing jobs; new submissions use `batch`.
