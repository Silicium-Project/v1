# ADR 0011 — Analysing untrusted files in WASM on Windows

**Status**: decided, not implemented.

## Context

On Linux, rendering runs in a strong sandbox (landlock, resource limits, `sil-sandbox-launch`). On Windows,
isolation is "best-effort": only the stop of the process tree is guaranteed. The remaining risk is a bug in the
parsing of an untrusted file (`.cfg` scene, image).

## Decision

On Windows, the parsing of untrusted files runs in a WASM module, not in the native process. The module has no
access to the file system, the network or the clock, and receives memory and time limits. A parsing bug stays
confined to the module.

- The crate `sil-render-wasm` holds this module. The test `tests/raytracer_parity.rs` checks that the WASM result
  is identical to the native rendering.
- The Windows node calls the module for parsing, then passes the validated data to rendering.
- Linux keeps the existing sandbox.

## Consequences

- The current limit ("an unverified render is not isolated on Windows") disappears once the module is in place for
  all the workloads concerned.
- Cost: WASM rendering is slower than native. Result parity is a condition, not speed.
- Rendering itself can stay native if only parsing is isolated; to be decided from measurements.

## Open points

- List of workloads whose parsing moves to WASM (`rendu/raytracer`, `image/batch`).
- Chosen WASM engine and memory and time limits.
