# Runtime and Factor Engine Guide

Finkit's runtime/factor layer is designed for repeatable, aligned factor computation over market data. It is intentionally narrower than a trading engine: it validates data shape and factor dependencies, executes computations, and returns aligned outputs.

## 1. Core concepts

### MarketFrame

`MarketFrame` represents aligned market series. The runtime expects related series to describe the same bars in the same chronological order.

Key rules:

- all required series must have compatible lengths;
- data is ordered oldest -> newest;
- aliases/canonical field names are resolved by the runtime contract rather than ad-hoc string handling;
- invalid shape/plan combinations are rejected rather than silently reindexed.

### Factor definitions

Factors represent reusable calculations that may depend on:

- raw market series;
- indicators;
- other factors;
- factor transforms.

### FactorPlan

A factor plan resolves dependencies before execution. Cyclic dependencies and missing dependencies are invalid.

### ComputePlan

The compute layer provides a unified execution path for reusable computations and output contracts. Plans should be compiled/validated once and reused when processing repeated frames or requests.

## 2. Intended execution flow

The production flow is:

1. prepare aligned OHLCV/market series;
2. construct a `MarketFrame`;
3. register or resolve factor definitions;
4. compile/validate the dependency graph;
5. execute the plan;
6. consume aligned outputs;
7. preserve warm-up `NaN` regions until all downstream inputs are valid.

This avoids hidden realignment and repeated dependency discovery.

## 3. Warm-up and alignment

Factors frequently depend on rolling indicators. Their leading output may therefore be `NaN`.

When combining outputs:

- preserve original bar indexes;
- build a joint finite-value mask;
- do not independently drop warm-up rows from each series;
- treat a non-finite value after the valid region starts as a condition worth investigating unless the specific algorithm documents it.

The same rule applies across Python, Rust, C/C++, Java, Node.js, and CLI consumers.

## 4. Dependency safety

The runtime validates factor dependencies instead of accepting ambiguous graphs.

Invalid examples include:

- empty factor identifiers;
- references to missing factors;
- duplicate/colliding registrations where the registry contract forbids them;
- dependency cycles such as `A -> B -> A`;
- stale plans used against incompatible runtime/schema state.

Plan validation should happen before putting a computation into a long-running service.

## 5. Reuse and performance

For high-throughput workloads:

- compile plans once;
- reuse runtime buffers where the public API permits it;
- avoid rebuilding registries/dependency graphs per bar;
- use incremental formula/streaming APIs when only the latest bar changes;
- benchmark the exact workload and target CPU rather than relying on universal throughput claims.

Finkit's CI includes zero-allocation and relative-performance gates for selected hot paths, but production performance remains workload- and machine-dependent.

## 6. Formula plans and factor plans

Formula and factor execution serve different purposes:

- use `CompiledFormula`/formula plans for terminal-style formula expressions and repeated formula evaluation;
- use factor plans for named dependency graphs composed from reusable factors and transforms;
- use streaming indicators for one-bar-at-a-time stateful updates when the computation naturally maps to an incremental indicator.

These can coexist in one application, but their ownership and update models should remain explicit.

## 7. Data ownership

Python zero-copy formula evaluation borrows contiguous `float64` arrays for the duration of the synchronous call. Do not resize or mutate those arrays concurrently while borrowed.

C/C++ consumers must follow the ownership/lifetime contract in [ffi/memory-contract.md](ffi/memory-contract.md).

For formula ownership/range/append semantics, see [formula-runtime-contract.md](formula-runtime-contract.md).

## 8. Error handling

Prefer explicit failure over silent correction. Typical validation failures should be handled at the boundary where a plan/frame enters the runtime.

For language-neutral/native error information, see [ffi/error-codes.md](ffi/error-codes.md).

## 9. Testing factor/runtime integrations

A good integration test should cover:

1. valid aligned data;
2. insufficient lookback/warm-up;
3. inconsistent input lengths;
4. missing dependency;
5. dependency cycle;
6. repeated execution of the same plan;
7. schema/registry mismatch when relevant;
8. numerical output alignment with a known reference case.

Repository-level validation includes core tests, Clippy, docs, version consistency, memory/performance gates, and multi-language package smoke tests.

## 10. Related documents

- [core-contracts.md](core-contracts.md) — canonical API/validation contracts
- [formula-runtime.md](formula-runtime.md) — reusable formula execution
- [formula-runtime-contract.md](formula-runtime-contract.md) — ownership and incremental semantics
- [architecture/dataflow.md](architecture/dataflow.md) — execution data flow
- [architecture/overview.md](architecture/overview.md) — crate/binding architecture
- [generated/formula-functions.md](generated/formula-functions.md) — generated function catalog
