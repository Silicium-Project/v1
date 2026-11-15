# ADR 0013 — Multi-dimensional Bayesian reputation and probabilistic claim assignment

**Status**: partially implemented. Replaces and extends `sil-reputation` (local scalar score, ADR 0003) and the
on-chain verifier score (ADR 0006) for everything that does not directly touch money. Penalties and weightings
that touch money remain governed by ADR 0006, 0008, 0010 and 0012: this ADR does not change them.

Done: phase 1 (deterministic Bayesian primitives, crate `sil-bayes` — `BetaPosterior`, `NormalGammaPosterior`,
`exp`/`ln`/`√`/`Φ` in fixed-point integers) and phase 2 (resource vector per workload, `sil-workload-spec::Resources`,
derived from the static descriptor of the workload rather than a field signed per job — see § 3). Remaining:
phases 3 to 10, blocked for phases 3 to 7 by ADR 0009 (not implemented) and by the numerical values listed under
Open points.

## Context

The network today has only one scalar score per worker, local to each node (`sil-reputation`), and a one-dimension
on-chain verifier score (ADR 0006). Neither captures the real diversity of machines (network speed, computation,
storage, model inference) nor the distinct skills (task delegation, detection of false verification, data
retention). The need, expressed by the requester:

> The scores of every agent on the network must be broken down by specialization. Each worker holds the full set
> of scores, and learns over time to specialize through Bayesian adjustment of the values. The scores include,
> among others: network speed (ping, acceptance time, upload/download throughput...), success rate, computation
> speed, quality of task delegation, quality of verification (detecting false verification), ability to store
> correctly over time (read/write speed, disk type, retention rate after audit). The network must also evaluate
> specific capabilities (model inference, GPU speed, RAM/VRAM...), ideally without explicitly naming task types.

This discussion settled, point by point, the shape this system must take. This ADR gathers them.

## Decision

### 1. Scores are shared and derived from the ledger — never a local opinion

A score that does not give the same result on every node cannot govern any decision that touches allocation or
money (ADR 0005 already states this principle for verifier reputation). Each score dimension is therefore a
**deterministic** function of the signed events of the ledger: same events, same total order, same result
everywhere. A node may keep a local opinion for choices with no stake (order of polling peers), but it never
enters an allocation, a penalty or a payment.

"Each worker holds the full set of scores": from the same ledger, each node rebuilds the score of every agent on
every dimension. There is no central table.

### 2. Measurements are verified, never declared

A worker must never be able to raise its own score by declaring a value. Two valid sources:

- **Measurement derived from the result of a real job**: observed throughput = units processed (scale in
  `sil-chain-core::units`) divided by a duration **anchored on-chain** (a Solana slot, ADR 0009), never the local
  clock of the worker or of a peer observing it alone.
- **Probe or challenge with a known answer, launched by a peer**: for dimensions a real job does not directly cover
  (pure network latency, raw throughput, disk read/write). The probing peer signs the observation as an event of
  the ledger.

No self-declared value (announced VRAM, announced disk type) enters a score until it is corroborated by a
measurement of this kind.

### 3. Specialization by resource vector, not by named task type

To stay general ("without explicitly naming task types"), each job carries a resource vector it draws on (GPU/CPU
profile, required RAM/VRAM, network intensity, storage intensity — quantities already present or derivable from
`EscrowTerms`/`JobSpec`, not a workload label). The network learns the relation **resource vector → observed
result** per agent, through a hierarchical model: a global prior per dimension, refined per agent as its
observations accumulate. A new workload needs no new category: it is only a new point in the vector space already
modeled.

**Implemented (phase 2), with a deliberate simplification**: rather than a new field signed per job (which would
have changed the on-chain escrow format, ADR 0009 not yet in place to absorb one more break), the resource vector
(`compute`, `memory`, `network`, `storage`, 0 to 100) is carried by the **static descriptor of the workload**
(`contracts/workloads/*.json` → `sil-workload-spec::Resources`, one per workload, never an aggregate). The scoring
layer never reads the workload identifier, only this vector: the constraint "without naming the task type" is
respected at the level where it matters (scoring), at the cost of a granularity per workload rather than per job
instance. Revisit this choice if one workload must someday draw very different profiles depending on its input.

### 4. Bayesian model: a conjugate family per kind of measurement

| Kind of measurement | Model | Why |
|---|---|---|
| Rate (success, false verification, retention after audit) | **Beta-Bernoulli** | Bounded to [0,1] by construction; exact even with a small sample (tens of observations) or a rate near 0/1. The Gaussian was rejected for this case: it overflows the interval and underestimates the tail (comparison traced in session, Beta(20,2) versus a Gaussian with the same mean and variance). |
| Continuous measurement (throughput, latency, computation speed) | **Normal-Gamma** (Gaussian with unknown variance) | Conjugate, streaming update (`τₙ = τ₀ + n·τ`, `μₙ = (τ₀μ₀ + τ·Σxᵢ)/τₙ`), predictive Student-t distribution (robust to outliers with a small sample). |
| Specialization (link resource → result) | **Hierarchical prior** over the parameters above, per agent, informed by a population prior | A new agent inherits the global prior; it specializes as its own observations come to dominate the prior. |

All transcendental functions this model needs (regularized incomplete beta function for `P(rate > threshold)`,
`Φ` for the Gaussian case) go through a **fixed table in fixed-point arithmetic**, compiled into the code: two nodes
with the same inputs must produce the same integer, never a floating-point value that depends on the platform.

### 5. Power → result diagrams, one per dimension

For hardware capabilities (computation speed, GPU, throughput, storage), the network builds, per dimension, a
diagram linking observed power to observed result (an ADR proposed in session, confirmed). This diagram:

- is built on **verified** measurements (point 2), never declared ones;
- gives a **learned envelope** (quantiles), not a line fixed in advance;
- serves to detect a gap between announced power and actual result (a score signal), **not** to reject a result:
  rejecting a computation remains a verification decision (ADR 0002, 0007), not a statistical one;
- exists in several versions, one per type of reputation/dimension (computation, storage, network, inference...),
  never a single diagram aggregating everything.

### 6. The score is a prior; only extreme values weigh directly

A score is computed — and weighs on a decision — only after a fixed minimum number of evaluations; before that
threshold the agent stays in **probation** (see § 8) and the score does not count. Beyond it, the score serves as a
**prior** for the decision (allocation, quota), except when the posterior indicates an extreme value (near-certain
fraud, near-certain incompetence): in that precise case, the score overrides the default rule.

The degree of certainty used to judge "extreme" is the posterior probability itself (`P(rate > threshold)` via
Beta, or the Gaussian certainty score), never a local opinion.

### 7. Dynamic thresholds depending on the state of the network — deterministic, bounded, smoothed

The trigger thresholds (what counts as "extreme", what opens or tightens a quota) are regulated by the state of the
network: a network under defense (high dispute rate, drop in active verifiers) tightens; a depleted network (few
workers, few verifiers) relaxes.

Conditions adopted:

- **The network state is itself a deterministic function of the ledger** (never a local measurement), built on
  signals that are hard to manipulate artificially (dispute rate **relative to the number of active verifiers**,
  not a simple count of events).
- **Basic linear regulation, bounded (clamp)**, with a floor and a ceiling hard-coded per category, completed if
  needed by a **sigmoid in a fixed table** (fixed-point) when a smoother transition is useful. Any form based on an
  exponential goes through a fixed table, never `f64::exp` directly.
- **Temporal smoothing** (no abrupt jump from one round to the next) is mandatory.

### 8. Bootstrapping a new agent: probation, then controlled exploration

An agent without history stays in probation (a single active claim, like the current `max_parallel` of
`sil-reputation`). Once the minimum number of evaluations (§ 6) is reached, its quota and its weight in the draws
open progressively. The exploration of its specialization (which dimensions it serves well) follows a Thompson-style
bandit: from time to time it receives claims outside its learned profile, to update its posterior without starving it
or favoring it indefinitely on a single profile.

### 9. Claim assignment: n independent lotteries, not a race

Each claim undergoes its own independent draw — there is **no** race between claims to be settled by a third party nor
by a single adversarial lottery. For a given claim:

```
accepted if hash(slot_seed ‖ claim_id) < p(score)
```

- **Seed**: a Solana slot posterior to the issue of the claim (ADR 0009). Nobody chooses it; a third-party drawer was
  rejected (it adds a point of trust — censorship by omission — without contributing anything the slot seed does not
  already give).
- **`p(score)`**: built from the posterior (§ 6), with a **fixed floor** leaving a non-zero chance to the smallest
  agent (so that a low score does not exclude it entirely, which would contradict the principle "prior unless
  extreme").
- **Several accepted claims on the same fragment**: the first accepted in total order (`issued_at_ms, id`, ADR 0005)
  takes the fragment; the others stay eligible for the next draw for another attempt.
- **Liveness**: `p` grows with the waiting time of the fragment, reaching 1 after a fixed duration, so that a fragment
  is never blocked for lack of a favorable draw.

### 10. Identifying and targeting fragments (parallelism without collision)

To exploit parallelism without a coordinator and without costly collisions:

- **Deterministic preference order per worker**: `hash(worker_key ‖ job ‖ fragment)` sorts the open fragments for this
  worker. Distinct workers thus aim, without coordinating, at different fragments first.
- **Rank sweep**: a worker idle on its preferred fragment (already taken, or refused by the draw of § 9) moves to the
  next one in its order.
- A collision remains possible but rare, and costless: only one claim wins per (fragment, attempt), the others pay
  nothing (ADR 0005, already in place).

### 11. Limiting multiple claims without an entry cost

A fixed cost per claim is prohibitive for the very first claim of an agent without history (confirmed in session).
The chosen limit combines, without ever requiring an initial payment:

- **Quota of simultaneous active claims**, not only a time window. A new agent has only one (probation, § 8). The
  quota grows with the posterior reputation.
- **Quota derived from the power → result diagram (§ 5)**, once the confidence threshold (§ 6) is reached:
  `quota = min(ceiling, floor(low_throughput × window / average_fragment_units))`, where `low_throughput` is the **low
  quantile** (not the mean — a lucky agent on one observation must not obtain a quota it has not proven) of this
  agent's diagram on this dimension.
- **Refundable bond rather than a cost** for claims beyond the first: locked at the claim, returned on successful
  verification. Zero for the first claim of a new agent.
- The existing per-task ceiling (`max_parallel`, `sil-reputation`) continues to apply on top of this quota.

Limit acknowledged and not resolved by the above: a population of disposable keys (sybils) can still accumulate free
first claims in parallel on distinct identities. The quota and the probation bound the damage per identity; they do
not prevent it at the scale of a key population. Treated as an open point (§ Open points).

## Consequences

- **Format**: new dimensions of signed events (peer probes/challenges, resource vectors of jobs). Incompatible change
  for any node that computes a score: simultaneous deployment required, as for ADR 0009/0010/0012.
- **Strengthened dependency on ADR 0009**: the slot seed and the anchoring of durations are direct prerequisites of
  this system (§ 2, § 9). This ADR cannot be implemented before ADR 0009.
- **Computation cost**: Bayesian updates and fixed tables add computation per node, bounded and deterministic (no
  transcendental `f64` directly).
- **`sil-reputation` is replaced** for everything that governs allocation (claims, quotas, specialization). It can
  remain as a pure local opinion, with no effect on allocation.
- The on-chain verifier score (ADR 0006) and the penalties (ADR 0008, 0010, 0012) do not change: they remain the only
  path that touches money directly.

## Open points

- Minimum number of evaluations before leaving probation (§ 6, § 11).
- Hard-coded certainty threshold for "extreme value" (§ 6) and for opening the diagram quota (§ 11), per category.
- Floors and ceilings per category for the dynamic thresholds (§ 7), and smoothing duration.
- Exact form of `p(score)` (§ 9): linear clamp or sigmoid in a table, and its floor for the smallest agent.
- Duration until `p = 1` (liveness, § 9) and window duration of the diagram quota (§ 11).
- Exact signals that define a "defensive" or "depleted" network, per category (§ 7).
- Exact resource vector carried by a job (which dimensions, what granularity) — to be defined with `sil-workload-spec`.
- Anti-sybil protection at the scale of a key population (acknowledged as unresolved, § 11): deferred entry cost, light
  proof of work, or a link to an external identity — to be decided separately.
- Exact set of probes/challenges per dimension (network, disk, GPU) and their frequency.

## Implementation plan

Dependency order: ADR 0009 (slot anchoring) is a hard prerequisite of phases 3 to 7 (every duration measurement and
every draw seed depends on it). Phases 1 and 2 are independent and can start immediately.

1. **Deterministic Bayesian foundations** — **done**: crate `sil-bayes` (`BetaPosterior`, `NormalGammaPosterior`,
   module `fixed`: `exp_micro`, `ln_micro`, `sqrt_micro`, `phi_micro`, all in `i128`, no system transcendental
   function). Exact closure tests (Beta(1,1) linear, Beta(2,1) quadratic, Beta(a,a) symmetric at 0.5) and determinism
   tests.
2. **Resource vectors of workloads** — **done**, with the simplification of § 3: a `resources` field in
   `contracts/workloads/*.json`, validated and emitted by `tools/workloads/generate.mjs` to
   `sil-workload-spec::Resources` (Rust), `workloadspec.Resources` (Go) and `WorkloadSpec` (TypeScript, field
   `resources`).
3. **Signed probes and challenges** — new ledger event type (a peer's observation of another: latency, throughput,
   disk read/write), signature, acceptance rules (indistinguishable from a real job, frequency).
4. **Power → result diagrams** — construction per dimension from verified measurements (phases 1 and 3), computation of
   the low quantiles, one diagram per dimension (not a single aggregate).
5. **Deterministic network state and dynamic thresholds** — function of the ledger (dispute rate / active verifiers,
   etc.), clamp regulation plus sigmoid in a table, temporal smoothing, bounds hard-coded per category.
6. **Probabilistic claim draw** — `hash(slot_seed ‖ claim_id) < p(score)`, `p` increasing with the waiting time
   (liveness), integrated into the current claim flow (ADR 0005) without a third-party drawer.
7. **Quotas and probation** — active-claim quota derived from the diagram (low quantile) once the confidence threshold
   is reached, refundable bond beyond the first claim, one-claim probation for a new agent, integration with the existing
   `max_parallel`.
8. **Deterministic targeting of fragments** — preference order per worker (`hash(key ‖ job ‖ fragment)`) and rank sweep
   on the node side (`sil-node`).
9. **Exploration bandit (bootstrapping)** — Thompson sampling to propose, from time to time, claims outside the learned
   profile of a new or weakly specialized agent.
10. **Removal of the attributive use of `sil-reputation` v1** — the old local scalar score stops weighing on any
    allocation; it can remain as a display opinion only.

## Related issues

See the GitHub repository: one issue per phase above, prefixed `reputation:`, referencing this ADR.
