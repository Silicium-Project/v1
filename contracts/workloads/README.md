# Workload descriptors

One file per workload: `contracts/workloads/<name>.json`. This is the only place a workload is
described. `node tools/workloads/generate.mjs` validates every descriptor and writes the generated
files in the component repositories (`--check` fails when one is stale):

| Consumer | Generated file | What it takes |
|---|---|---|
| Node and chain (Rust) | `network/sil-workload-spec/src/generated.rs` | unit, limits, verification, split, terms |
| API (Go) | `services/api/internal/workloadspec/generated.go` | everything, including `ui` and `closed` |
| Website (TypeScript) | `apps/web/src/workloads/generated.ts` | everything, including `ui` and `closed` |

Nothing in the API or the website lists workload identifiers. The submission form, the shape of the
result and whether new jobs are accepted all come from the descriptor.

## Fields

```json
{
  "schema": "silicium.workload/v1",
  "id": "batch/classify",
  "description": "Text shown in the generated documentation.",
  "unit": "bytes_input",
  "label_key": "submit.workloadClassify",
  "input": { "params": [], "max_input_bytes": 8388608 },
  "split": { "max_fragments": 16, "target_size": 262144 },
  "terms": { "reward_per_unit": 100, "verifier_reward_per_unit": 10 },
  "verification": { "policy": "sampling_tolerance", "min_cheat_fraction": 0.1, "target_detection": 0.97,
                    "cos_min": 0.999, "model_sha256": "…", "ort_version": "1.31.1" },
  "limits": { "wall_ms": 300000, "memory_bytes": 536870912, "output_bytes": 67108864 },
  "deposit": "supported",
  "resources": { "compute": 70, "memory": 40, "network": 20, "storage": 30 },
  "closed": true,
  "ui": { "input": "images", "options": false, "result": "classification" }
}
```

- `unit`: `bytes_input` (one unit = one input byte) or `pixels_band`. A new unit needs an on-chain rule.
- `verification.policy`: `recompute`, `sampling` or `sampling_tolerance` (the last one pins the model
  by SHA-256 and the ONNX Runtime version).
- `closed` (optional, default false): the descriptor is kept for jobs already published, but the API
  and the website open no new job for it.
- `ui.input`: the submission form. `file` (any file), `scene` (the scene editor, for `pixels_band`),
  `images` (a batch of images), `documents` (a batch of PDFs), `texts` (files whose text is extracted).
- `ui.options` (optional, only with `images`, default true): the image form shows its resize options.
  `false` for a workload that has none.
- `ui.result`: the shape of the result. The API hands it over in this shape and the website shows it:

| `ui.result` | API gives | Website shows |
|---|---|---|
| `text` | text file: raw fragment results and their sha256 | download (TXT) |
| `image` | PNG assembled from the fragments | download (PNG), preview |
| `files` | ZIP, one file per image | download (ZIP) |
| `documents` | ZIP, one text file per document (`0003.illisible.txt` when unreadable) | download (ZIP) |
| `embedding` | raw f32 vectors, 384 per passage | table, export, search |
| `classification` | raw f32 scores, 1 000 per image | table with the top categories |

## Add a workload

1. Write `contracts/workloads/<name>.json`, then `node tools/workloads/generate.mjs`.
2. Rust: a crate `sil-wl-<name>` that implements `Workload` (`split`, `execute`, `verify` or
   `verify_tolerance`, `combine`), registered in the node (`sil-node`). A pinned model goes through
   `sil_model::ensure_entry`.
3. Texts: `label_key` in `fr.ts` and `en.ts`, and an entry in the public catalogue (`catalogue.ts`
   and the `workloadsPage.items` of `fr-site.ts` and `en-site.ts`).
4. Reuse an existing `ui.input` and `ui.result` when one fits: the API and the website then need no
   change. A new result shape adds a value in `ResultKind` (Go), in the generator, and a download
   entry (`workloads/result.ts`) or a component in the website.
5. Tests: `cargo test --workspace`, `go test ./...` and `npm test`. The tests over every descriptor
   (`TestResultShapeFollowsTheDescriptorOfEveryWorkload`, `workloads.test.ts`) cover the new workload.
