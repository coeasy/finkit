# Finkit Documentation

This directory is the canonical documentation set for Finkit.

Finkit is a Rust-powered quantitative finance engine for technical analysis, reusable formulas, factor graphs, streaming computation, feature engineering, and research workflows. The documentation deliberately separates **published v0.1.15 distribution facts** from **next-release runtime/research work** so CI candidates are never presented as already released packages.

## Start here

Choose the path that matches your goal:

| Goal | Start with |
| --- | --- |
| Understand the product | [Product overview](product-overview.md) |
| 中文产品介绍 | [中文产品说明](product-overview-zh.md) |
| 宣传/项目介绍素材 | [中文宣传文稿](promotion-zh.md) |
| Install and calculate something | [Getting started](getting-started.md) |
| Verify install/release assets | [Installation](installation.md) |
| Learn the end-to-end APIs | [Complete usage guide](usage.md) |
| Use Python | [Python guide](python.md) |
| Use the CLI | [CLI guide](cli.md) |
| Understand language/package status | [Language bindings](language-bindings.md) |
| Build factor/runtime workloads | [Runtime and factors](runtime-and-factors.md) |
| Understand research architecture | [Factor research architecture](factor-research-architecture.md) |
| Diagnose failures | [Troubleshooting](troubleshooting.md) |

## Product model

Finkit is organized around one core idea: **financial semantics should be canonical and reusable across execution modes and languages.**

```text
Indicators / Formula / Factors / Streaming
                 │
                 ▼
        canonical kernels + plans
                 │
                 ▼
            Unified Runtime
                 │
        ┌────────┴────────┐
        ▼                 ▼
    Features          Research
        │                 │
        └────────┬────────┘
                 ▼
Rust / Python / CLI / native bindings / WASM
```

The project is a calculation/research engine, not an OMS, brokerage connector, exchange gateway, matching engine, or turnkey live-trading platform.

## Published v0.1.15 contract

The GitHub `v0.1.15` Release is the authoritative distribution contract for the current published version.

Its release assets include:

- four Python `cp38-abi3` wheels for Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64;
- `finkit-0.1.15.crate`;
- `finkit-cli-linux-x86_64`;
- `SHA256SUMS`.

Public package registries are a separate contract. Do not assume PyPI, crates.io, npm, Maven Central, NuGet, a public Go module, Android Maven coordinates, or Swift package coordinates are available unless that exact package/version has been published and smoke-tested.

## Next-release architecture

The active next-release work expands the reusable runtime and factor-research stack. The key architectural changes include:

- typed, deterministic `ArtifactHash` identity;
- `FactorPlan` execution through the shared `UnifiedRuntime` boundary;
- `DirtyRange` invalidation with explicit input-dirty, affected-output, and historical recompute ranges;
- local execution only for dependency chains that are proven incremental, fixed-lookback, and time-series safe;
- conservative full fallback for cross-sectional, dynamic-lookback, or unknown execution contracts;
- retained materializations for caller-owned range/into updates;
- reuse-first Factor Research integration built on canonical returns/statistics/regression/rank/risk/calendar infrastructure.

These capabilities are next-release development facts and are not retroactively claimed as published v0.1.15 assets.

## Multi-language validation model

Finkit distinguishes five states:

1. **source exists**;
2. **CI validated**;
3. **package candidate**;
4. **GitHub Release asset**;
5. **public registry package**.

The expanded multi-language workflow includes target-specific validation paths for Go/CGO, .NET, WASM, Android, iOS, Node.js, Java/JNI, C/C++, Rust/CLI, and Python packaging.

A target is only considered CI validated after its final-head job actually runs and passes. A green job from another SHA is not evidence for the current release candidate.

## User guides

| Document | Purpose |
| --- | --- |
| [getting-started.md](getting-started.md) | Fast path from installation to verified calculations |
| [installation.md](installation.md) | Release assets, prerequisites, source builds and installation verification |
| [usage.md](usage.md) | End-to-end usage patterns and data/runtime conventions |
| [python.md](python.md) | ABI3 wheels, NumPy, `CompiledFormula`, pandas and troubleshooting |
| [cli.md](cli.md) | CLI input formats and indicator/formula/streaming commands |
| [language-bindings.md](language-bindings.md) | Binding, package-candidate and publication support matrix |
| [runtime-and-factors.md](runtime-and-factors.md) | Unified Runtime, FactorPlan, DirtyRange, dependency safety and reuse |
| [troubleshooting.md](troubleshooting.md) | Failure isolation across install/runtime/native build paths |
| [indicators.md](indicators.md) | Human-readable indicator reference |
| [features.md](features.md) | Feature engineering API/module guide |

Binding-specific source guides also live with their implementations, including `ffi/go-binding/README.md`, `ffi/dotnet-binding/README.md`, `ffi/android-binding/README.md`, `ffi/ios-binding/README.md`, and `wasm/README.md`.

## Formula system

| Document | Purpose |
| --- | --- |
| [formula.md](formula.md) | Formula syntax, evaluation and debugging guidance |
| [formula/grammar.md](formula/grammar.md) | Core formula grammar |
| [formula/pine-grammar.md](formula/pine-grammar.md) | Supported Pine grammar subset |
| [formula-runtime.md](formula-runtime.md) | Persistent compiled plans and incremental execution |
| [formula-runtime-contract.md](formula-runtime-contract.md) | Ownership, `eval_range`, `eval_last`, append, warm-up and concurrency semantics |
| [formula-templates.md](formula-templates.md) | Reusable formula patterns |
| [formula-performance.md](formula-performance.md) | Formula optimization and benchmark notes |
| [migration/pine-to-finkit.md](migration/pine-to-finkit.md) | Pine migration guidance and semantic boundaries |

For exact supported functions and Pine mappings, prefer generated catalogs over hard-coded counts or compatibility percentages.

## API and architecture

| Document | Purpose |
| --- | --- |
| [api-reference.md](api-reference.md) | Public API overview |
| [api-reference-zh.md](api-reference-zh.md) | 中文 API 参考 |
| [core-contracts.md](core-contracts.md) | ComputePlan, FactorPlan, MarketFrame and registry contracts |
| [function-schema.md](function-schema.md) | Versioned machine-readable function schema |
| [architecture/overview.md](architecture/overview.md) | Crate/binding architecture |
| [architecture/dataflow.md](architecture/dataflow.md) | Batch, streaming, formula and binding data flow |
| [architecture/formula-engine.md](architecture/formula-engine.md) | Formula parser/compiler/runtime internals |
| [factor-research-architecture.md](factor-research-architecture.md) | Reuse-first Factor Research / Alphalens-style / multi-factor / portfolio architecture |
| [ffi/memory-contract.md](ffi/memory-contract.md) | C ABI ownership/lifetime contract |
| [ffi/error-codes.md](ffi/error-codes.md) | Cross-language/native error codes |

## Performance and engineering quality

| Document | Purpose |
| --- | --- |
| [benchmark-results.md](benchmark-results.md) | Current benchmark summary |
| [BENCHMARK_VS_TALIB.md](BENCHMARK_VS_TALIB.md) | TA-Lib comparison methodology |
| [BENCHMARK_REPORT.md](BENCHMARK_REPORT.md) | Generated benchmark snapshot |
| [FUZZING.md](FUZZING.md) | Fuzz targets and crash reproduction |
| [development.md](development.md) | Build, test, benchmark, package and CI workflow |

Benchmark values are measured snapshots, not universal latency/throughput guarantees. Re-run the benchmark harness on the target CPU/compiler/runtime before making production commitments.

## Generated source of truth — do not delete

The following files are generated or machine-readable contracts and are intentionally retained:

- `indicator_registry.json` — canonical indicator registry snapshot;
- `generated/indicators.md` — generated indicator catalog;
- `generated/streaming-indicators.md` — generated streaming registry;
- `generated/formula-functions.md` — generated formula function list;
- `generated/features.md` — generated feature matrix;
- `generated/error-codes.md` — generated error-code reference;
- `generated/pine-compatibility.md` — generated Pine compatibility matrix;
- `generated/version-matrix.md` — generated release/version matrix;
- `benchmark-baseline.json` — performance-gate baseline where used by CI/scripts.

`scripts/gen_ssot_docs.py --check`, `scripts/check_versions.py`, and `scripts/check_docs_links.py` protect these contracts.

## Documentation rules

Active documentation should not contain:

- completed release plans presented as current product behavior;
- stale PR/progress snapshots;
- duplicate implementation notes when a canonical guide exists;
- unverified public-registry install commands;
- hard-coded capability counts already available from SSOT;
- examples that call APIs absent from the relevant binding;
- CI package candidates written as if they are published distributions;
- performance numbers presented as universal guarantees;
- incremental-execution claims that ignore range-safety constraints.

When code or release behavior changes:

1. update the user-facing guide that owns the behavior;
2. update root README / product overview if the product contract changes;
3. update binding-specific docs together with package metadata;
4. regenerate SSOT docs instead of hand-editing generated files;
5. run link/version/SSOT checks;
6. distinguish source, CI validation, candidates, release assets, and public packages;
7. verify every public API used in an example exists in that binding.

Recommended validation:

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
cargo fmt --all -- --check
cargo test --workspace --doc --locked
```

_Last product/documentation review: 2026-09-12. Published distribution baseline: v0.1.15._
