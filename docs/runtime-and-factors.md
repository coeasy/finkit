# Runtime and Factor Engine Guide

Finkit's runtime/factor layer provides repeatable, aligned, dependency-safe financial computation over market data. It is designed as a reusable calculation boundary for services, research systems, scanners, and SDKs — not as a trading engine.

The current architecture is converging factor and research execution on the same runtime contracts used for reusable computation: compile dependencies once, validate execution semantics once, reuse retained outputs when safe, and fall back conservatively when local execution cannot be proven correct.

## 1. Core concepts

### MarketFrame

`MarketFrame` represents aligned market series. Related fields must describe the same bars in the same chronological order.

Key rules:

- all required series must have compatible lengths;
- data is ordered oldest -> newest;
- aliases/canonical field names are resolved by the runtime contract rather than ad-hoc caller logic;
- invalid shape/plan combinations are rejected rather than silently reindexed;
- warm-up and missing-value semantics remain explicit.

### FactorDefinition

A factor is a named reusable computation. It may depend on:

- raw market series;
- indicators;
- other factors;
- factor transforms.

A factor definition also carries execution semantics such as factor kind and, where available, incremental/lookback capability.

### FactorPlan

`FactorPlan` resolves the factor dependency graph before execution. It owns stable dependency-first execution order and required raw input discovery.

Compilation rejects invalid plans such as missing factors and dependency cycles. Reusing a compiled plan avoids repeatedly traversing the graph for every request.

### ComputePlan

`ComputePlan` is the general dependency/effect/capability planning contract used by reusable computations. Research orchestration reuses this infrastructure rather than implementing a separate topology sorter.

### UnifiedRuntime

`UnifiedRuntime` is the shared execution boundary for reusable planned computation.

The factor implementation currently exposes full and range-oriented execution through this boundary. The long-term goal is to keep Formula, Factor, Research, compute-many, and related retained-execution paths on compatible runtime semantics rather than creating isolated executors.

### ArtifactHash

`ArtifactHash` is a typed deterministic content identity for materialized runtime/research artifacts.

The important property is not the specific hash width; it is that artifact identity is explicit, stable, and cannot be confused with an arbitrary untyped integer field.

### DirtyRange

`DirtyRange` models invalidation as a half-open row range.

For correct local execution, three ranges must be distinguished:

1. **input dirty range** — source rows that changed;
2. **affected output range** — output rows that may change after propagating the dependency chain's lookback forward;
3. **recompute range** — the affected output range expanded backward by the historical lookback needed to calculate it.

For example, if one source bar changes and a dependency chain has an accumulated fixed lookback of 20, the changed bar can affect later rolling outputs. Reading 20 historical rows is not enough by itself; the output invalidation must also propagate forward.

## 2. Intended full execution flow

The baseline production flow is:

1. prepare aligned OHLCV/market series;
2. construct/validate a `MarketFrame` or factor context;
3. register/resolve factor definitions;
4. compile the factor dependency graph into `FactorPlan`;
5. validate that the runtime registry still matches the plan;
6. execute through `UnifiedRuntime`;
7. consume aligned outputs;
8. preserve warm-up `NaN` regions until downstream inputs are jointly valid.

This avoids hidden realignment and repeated dependency discovery.

## 3. DirtyRange local execution

Local execution is an optimization with a correctness proof requirement.

The runtime path is conceptually:

```text
changed source rows
       │
       ▼
 input DirtyRange
       │
       ▼
propagate forward through accumulated lookback
       │
       ▼
affected output rows
       │
       ▼
expand backward for historical inputs
       │
       ▼
 recompute window
       │
       ▼
execute only the required slice
       │
       ▼
splice affected outputs into retained materialization
```

### Range-safe requirements

A factor plan may use local range execution only when the complete dependency chain can prove the required behavior. The current safety boundary includes:

- time-series-compatible factor kind;
- explicit incremental capability;
- fixed/known lookback;
- no cross-sectional dependency that requires recomputing peers outside the local time range;
- no dynamic/unbounded history requirement;
- retained output arrays with the correct full-row shape.

If those conditions are not met, the caller/runtime must use full execution.

### Why fallback is intentional

A fast wrong answer is worse than a correct full recomputation. Finkit therefore treats unknown range semantics as `FullOnly`, not as an opportunity to guess.

Cross-sectional ranks, global normalizations, dynamic-window algorithms, and state whose provenance cannot be reconstructed from the local slice are examples that may require full fallback unless a stronger contract is implemented.

## 4. Full, borrowed, range and into paths

The runtime is designed around four useful execution shapes:

### Full

Compute the complete plan from authoritative inputs.

Use when:

- no retained output exists;
- the plan is not range-safe;
- a broad data revision invalidates most of the history;
- the caller prefers simple full materialization.

### Borrowed

Execute against borrowed aligned input without requiring an unnecessary ownership conversion.

Borrowed inputs must remain valid and must not be resized/mutated concurrently during synchronous evaluation.

### Range

Clone/return a retained output materialization after recomputing only the affected range plus historical input window.

Use when the plan is range-safe and immutable-style result ownership is convenient.

### Range into

Update retained output arrays in place.

Use when the caller already owns full materializations and wants to avoid cloning clean rows.

The `into` path is the preferred hot-path shape for long-running services when ownership and concurrency rules allow it.

## 5. Warm-up and alignment

Factors frequently depend on rolling indicators. Their leading output may therefore be `NaN`.

When combining outputs:

- preserve original bar indexes;
- build a joint finite-value mask;
- do not independently drop warm-up rows from each series;
- treat unexpected non-finite values after the valid region starts as conditions worth investigating unless the specific algorithm documents them.

The same data contract applies across Python, Rust, C/C++, Java, Node.js, CLI, and other bindings.

## 6. Dependency safety

Invalid examples include:

- empty factor identifiers;
- references to missing factors;
- duplicate/colliding registrations where the registry contract forbids them;
- dependency cycles such as `A -> B -> A`;
- stale plans used against a registry whose dependencies changed;
- range execution against missing or wrong-length retained outputs;
- a dirty range outside the input row count.

Plan validation should happen before a computation enters a long-running service.

## 7. Reuse and performance

For high-throughput workloads:

- compile plans once;
- reuse factor registries where appropriate;
- use borrowed input paths to avoid avoidable copies;
- prefer caller-owned `into` outputs on hot paths;
- keep retained materializations when local updates are common;
- avoid rebuilding dependency graphs per bar/request;
- use Streaming Indicator implementations for naturally stateful append-only calculations;
- use DirtyRange only when range safety is proven;
- benchmark the exact workload/CPU/compiler/binding rather than relying on universal throughput claims.

Finkit's CI includes zero-allocation, relative-performance, throughput, and multi-language validation gates for selected paths. Performance regressions are treated as real release signals rather than being hidden by relaxed thresholds.

## 8. Formula plans, factor plans and streaming state

These execution modes serve related but distinct use cases:

- `CompiledFormula` is suited to terminal-style expression execution and repeated formula evaluation;
- `FactorPlan` is suited to named dependency graphs composed from reusable factors;
- streaming indicators are suited to one-bar-at-a-time retained state;
- ResearchPlan orchestrates research stages while reusing core planning/canonical math.

They can coexist in one application. The architectural direction is shared runtime contracts and canonical kernels, not forcing every computation into an identical implementation.

## 9. Artifact identity and provenance

A research/runtime artifact should be identifiable from the inputs and semantics that produced it.

`ArtifactHash` provides the typed content-identity building block. Higher layers can add provenance fields such as:

- input revision/source identity;
- plan/schema version;
- factor definition version;
- execution policy;
- calendar/point-in-time configuration;
- dependency artifact hashes.

This is the basis for safe cache reuse and reproducible research. Cache keys should not be derived from opaque process-local state or unstable hashing behavior.

## 10. Research integration

The Factor Research layer should consume runtime outputs instead of owning another factor executor.

The intended chain is:

```text
MarketFrame / source data
       ↓
FactorPlan / Formula / canonical kernels
       ↓
Unified Runtime
       ↓
Feature / ResearchFrame / FactorPanelView
       ↓
Prepare / Validate / Analyze
       ↓
Multi-factor / Portfolio / Risk / Event
       ↓
Typed Research Artifacts / Report
```

Research-specific metadata can be stored independently, but topology sorting, cycle validation, core statistics, returns, regression, rank, risk, and calendar semantics should be reused from their canonical modules.

## 11. No-lookahead and point-in-time rules

Range execution is not the same thing as research safety.

Predictive research must separately preserve information boundaries:

- forward returns may use future prices only as labels/evaluation targets, never as factor inputs;
- normalization/neutralization/fitting must use the appropriate point-in-time sample;
- event/calendar alignment must not use future availability information;
- cached artifacts must include enough provenance to prevent reuse across incompatible information sets.

No-lookahead tests belong at research/runtime integration boundaries, not only in individual math functions.

## 12. Data ownership

Python zero-copy paths borrow contiguous `float64` arrays for the duration of the synchronous call. Do not resize or mutate those arrays concurrently while borrowed.

C/C++ consumers must follow [ffi/memory-contract.md](ffi/memory-contract.md).

Formula ownership/range/append semantics are documented in [formula-runtime-contract.md](formula-runtime-contract.md).

## 13. Error handling

Prefer explicit failure over silent correction.

Typical boundary errors include:

- missing input;
- incompatible lengths;
- invalid dirty ranges;
- missing retained outputs;
- stale dependency plans;
- unsafe range execution requests;
- invalid warm-up/NaN policy combinations.

For native error information, see [ffi/error-codes.md](ffi/error-codes.md).

## 14. Recommended runtime tests

A strong runtime/factor integration test should cover:

1. valid aligned full execution;
2. borrowed/full equivalence;
3. insufficient lookback/warm-up;
4. inconsistent input lengths;
5. missing dependency;
6. dependency cycle;
7. stale plan after registry dependency change;
8. repeated execution of the same plan;
9. range-safe local execution equal to full recomputation;
10. changed input propagating to future affected outputs;
11. historical lookback expansion for the local slice;
12. clean rows remaining unchanged after range-into execution;
13. cross-sectional/dynamic-lookback full fallback;
14. empty/out-of-bounds dirty range behavior;
15. retained-output shape validation.

## 15. Release-gate expectations

Runtime changes are not ready merely because they compile.

Final-head validation should include, as applicable:

- Rust workspace/check/Clippy;
- core tests;
- factor/research correctness tests;
- Runtime integration tests;
- performance/throughput gates;
- docs/SSOT/version checks;
- Python wheels;
- multi-language builds/package smoke tests.

Only results from the same final commit SHA should be used as release evidence.

## 16. Related documents

- [Product overview](product-overview.md)
- [core-contracts.md](core-contracts.md) — canonical API/validation contracts
- [factor-research-architecture.md](factor-research-architecture.md) — research architecture
- [formula-runtime.md](formula-runtime.md) — reusable formula execution
- [formula-runtime-contract.md](formula-runtime-contract.md) — ownership and incremental semantics
- [architecture/dataflow.md](architecture/dataflow.md) — execution data flow
- [architecture/overview.md](architecture/overview.md) — crate/binding architecture
- [language-bindings.md](language-bindings.md) — binding/publication status
