# Finkit Product Overview

## Product in one sentence

**Finkit is a Rust-powered quantitative finance engine that unifies technical analysis, reusable formulas, factor graphs, streaming analytics, feature engineering, and research computation behind one multi-language runtime.**

## The problem Finkit solves

Financial applications frequently start with a few indicator functions and gradually accumulate multiple independent calculation stacks:

- Python research code implements one version of an indicator while a production service implements another;
- formula engines, factor systems, feature pipelines, and streaming services each maintain their own dependency graphs and caches;
- repeated requests parse the same expression, rediscover the same dependencies, and allocate the same scratch buffers;
- "incremental" APIs still recompute full histories because invalidation semantics are not modeled explicitly;
- statistics, ranking, regression, returns, and risk calculations are copied into multiple modules;
- language bindings drift because each ecosystem becomes responsible for too much numerical behavior.

Finkit treats these as an architecture problem rather than a catalog-size problem.

The core design principle is:

> **Implement financial semantics once, compile dependencies once, reuse execution state, and expose the same contracts through every supported language.**

## Product boundaries

Finkit is a calculation and research engine. It intentionally does not own brokerage connectivity, OMS logic, exchange gateways, matching, live order routing, or account management.

That boundary lets the project focus on:

- numerical correctness;
- aligned time-series semantics;
- reusable execution plans;
- research safety and no-lookahead behavior;
- low-allocation/high-throughput computation;
- deterministic artifacts and reproducible analysis;
- cross-language API delivery.

## Product layers

### 1. Financial computation core

The Rust core contains canonical implementations for technical indicators, transforms, statistics, patterns, risk helpers, calendar logic, returns, feature engineering, and related numerical primitives.

Public APIs may provide convenient wrappers, but canonical kernels should not be independently reimplemented in multiple subsystems.

### 2. Formula engine

The formula layer supports terminal-style expressions and reusable compiled plans. It is designed for workloads where users want to define calculations declaratively and execute the same formula repeatedly over aligned market data.

The runtime includes optimized execution paths such as cached compilation, bytecode/JIT-oriented execution, range/latest evaluation, append-oriented updates, reusable buffers, and zero-copy/borrowed inputs where supported.

### 3. Factor engine

Factors are named reusable computations with explicit dependencies. `FactorPlan` compiles and validates the dependency graph before numerical execution, allowing repeated workloads to reuse dependency discovery and stable execution order.

The factor layer is moving from an isolated executor toward the same runtime boundary used by other reusable computation paths.

### 4. Unified Runtime

The next-release architecture establishes shared contracts for full, borrowed, range, and caller-owned output execution.

Key concepts include:

- `ArtifactHash` — typed and deterministic content identity;
- `DirtyRange` — explicit invalidation semantics;
- `FactorPlan` — prevalidated factor dependency execution;
- retained materializations — previously computed outputs that can be updated locally;
- conservative fallback — full execution whenever local recomputation cannot be proven correct.

Dirty-range execution is modeled as three different ranges:

1. **input dirty range** — source rows that changed;
2. **affected output range** — rows whose result may have changed after forward propagation through lookback dependencies;
3. **recompute range** — historical input window needed to recompute the affected outputs correctly.

This distinction prevents a common incremental-computing error: reading enough history but updating too few future rows.

### 5. Streaming engine

For calculations that naturally evolve one bar at a time, Finkit provides stateful streaming indicators. Batch/streaming parity tests are used where appropriate so realtime and historical paths do not silently diverge.

### 6. Feature and research layer

The project includes feature engineering, labels, cross-validation helpers, stability/regime tools, PCA, selection/importance primitives, lightweight backtest/risk components, and the developing Factor Research layer.

The research architecture is reuse-first: it consumes existing canonical returns, statistics, regression, rank, calendar, risk, feature, and execution infrastructure rather than creating another research-only math stack.

### 7. Multi-language delivery

Rust is the semantic center. Other languages consume the same core through bindings/package layers.

Targets include:

- Python;
- Rust;
- CLI;
- Node.js;
- Java/JNI;
- C/C++;
- Go/CGO;
- .NET;
- Android;
- iOS;
- WebAssembly.

Publication status varies by language. Finkit intentionally distinguishes source availability, CI validation, package candidates, GitHub Release assets, and public registry publication.

## Who Finkit is for

### Quant researchers

Use one toolkit for technical indicators, formulas, features, labels, factors, statistics, validation, and reusable research artifacts.

### Quant/platform engineers

Embed a high-performance calculation layer into data services, screening systems, signal services, analytics APIs, or research infrastructure without binding the engine to a broker.

### Realtime analytics teams

Combine retained streaming state with precompiled formula/factor plans for repeated market-data workloads.

### SDK and product teams

Keep numerical behavior in one Rust core while exposing native APIs to multiple runtimes and operating systems.

### ML/data teams

Use financial transforms, rolling statistics, feature generation, labels, validation primitives, and research outputs in reproducible data pipelines.

## Typical product scenarios

### Market scanner

Compile frequently used formulas/factors once, evaluate them against many symbols, preserve alignment, and expose results through a service API.

### Factor research service

Transform aligned market data into factors/features, validate forward returns and point-in-time rules, evaluate factor behavior, generate reports, and persist typed research artifacts.

### Realtime analytics backend

Use streaming indicators for append-only bars and DirtyRange-based recomputation for corrected historical data when the dependency chain is range-safe.

### Multi-language analytics SDK

Ship a common calculation engine to Python notebooks, JVM services, Node applications, C/C++ systems, mobile clients, and browser/WASM consumers.

## Product architecture

```text
                        ┌───────────────────────────┐
                        │ Market / External Inputs  │
                        └─────────────┬─────────────┘
                                      │
                                      ▼
                           MarketFrame / contracts
                                      │
          ┌───────────────────────────┼──────────────────────────┐
          │                           │                          │
          ▼                           ▼                          ▼
     Indicators                    Formula                    Factors
          │                           │                          │
          └──────────────┬────────────┴──────────────┬───────────┘
                         │                           │
                         ▼                           ▼
                  canonical kernels          Compute/Factor plans
                         │                           │
                         └─────────────┬─────────────┘
                                       ▼
                                Unified Runtime
                    full / borrowed / range / into / retained
                                       │
                 ┌─────────────────────┼─────────────────────┐
                 ▼                     ▼                     ▼
             Streaming             Features              Research
                 │                     │                     │
                 └─────────────────────┴────────────┬────────┘
                                                    ▼
                              Rust / Python / native SDKs / WASM
```

## Design principles

### Single source of truth

Generated registries and schemas describe supported functions/features. Documentation should consume those sources instead of maintaining hand-written counts that drift.

### Reuse first

New research or runtime modules should reuse existing math, graph, cache, calendar, risk, and buffer infrastructure before adding another implementation.

### Explicit semantics

Warm-up, NaN behavior, input alignment, lookback, statefulness, determinism, effects, and data ownership are contracts, not accidental behavior.

### Correct incremental execution

A local execution path must prove that every node in the dependency chain is compatible with range recomputation. Unknown/dynamic/cross-sectional semantics fall back to full execution.

### No-lookahead research

Forward-return analysis, validation, feature fitting, regime detection, event studies, and portfolio research must preserve point-in-time information boundaries.

### Multi-language parity

Bindings should expose core behavior, not fork it. Language-specific packaging can differ; numerical semantics should not.

## Performance strategy

Finkit optimizes both kernels and orchestration:

- SIMD-accelerated numerical paths;
- caller-owned `_into` buffers;
- zero-copy and borrowed input views;
- persistent compiled formulas;
- dependency-plan reuse;
- retained streaming state;
- BufferArena/scratch reuse;
- local DirtyRange execution for proven-safe plans;
- multi-period/fused kernels where they reduce redundant passes;
- CI performance/benchmark regression checks.

Performance figures depend on CPU, compiler, workload, feature flags, memory layout, and binding overhead. Production users should benchmark the exact target configuration.

## Reliability strategy

Release confidence is built from layered gates:

- unit and integration tests;
- batch/streaming equivalence checks;
- selected external/reference parity tests;
- formula semantic contracts;
- no-lookahead and point-in-time tests for research paths;
- Rust format/Clippy/workspace checks;
- SSOT/version/docs-link validation;
- no-std/feature-specific checks where relevant;
- Python ABI3 wheel builds;
- multi-language native packaging/runtime smoke tests;
- performance regression gates.

A workflow that does not actually execute its steps is not treated as evidence of correctness, and results from different commit SHAs are not combined into a synthetic “green” release state.

## Current release and next-release distinction

### Published v0.1.15

The GitHub v0.1.15 Release is the authoritative distribution contract for that version. It includes verified Python ABI3 wheels, a Rust crate asset, a Linux x86_64 CLI binary, and checksums.

### Next-release architecture

The active factor-research work expands the runtime/research architecture with typed artifacts, Unified Runtime integration, dependency-safe dirty-range execution, and broader research workflows. These capabilities are not retroactively claimed as v0.1.15 release assets.

## Roadmap

Near-term priorities:

1. finish Unified Runtime adoption across reusable execution paths;
2. harden DirtyRange correctness/performance gates;
3. complete Factor Research preparation/analysis/report workflows;
4. converge duplicate statistics, regression, returns, rank, quantile, and risk implementations onto canonical kernels;
5. connect research artifacts to deterministic cache/provenance contracts;
6. complete same-SHA cross-platform/multi-language release gates;
7. improve end-user product documentation, examples, and package discovery.

Longer term, Finkit should become a dependable computational substrate for quantitative products rather than an ever-growing collection of disconnected helper functions.

## Related documentation

- [Getting started](getting-started.md)
- [Runtime and factors](runtime-and-factors.md)
- [Factor research architecture](factor-research-architecture.md)
- [Formula runtime](formula-runtime.md)
- [Language bindings](language-bindings.md)
- [Development](development.md)
- [Benchmark results](benchmark-results.md)
- [Chinese product overview](product-overview-zh.md)
- [Chinese promotional copy](promotion-zh.md)
