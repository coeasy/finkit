# Finkit Documentation

This directory is the canonical documentation set for the current Finkit codebase. It is organized around what users can actually install and run from the `v0.1.3` baseline, with implementation details separated from user-facing instructions.

## Start here

1. [Installation](installation.md) — choose a Release asset or build a binding from source.
2. [Complete usage guide](usage.md) — Python, Rust, CLI, formulas, streaming, factors/runtime, and other bindings.
3. [Python guide](python.md) — ABI3 wheel selection, NumPy rules, reusable formulas, troubleshooting.
4. [Indicators](indicators.md) — indicator catalog and parameter reference.
5. [Formula engine](formula.md) — formula language and compatibility.
6. [Development](development.md) — build, test, benchmark, package, and CI workflow.

## Release and distribution status

Current source/release version: **0.1.3**.

The GitHub `v0.1.3` Release currently publishes the verified binary/source artifacts below:

- Python ABI3 wheel — Linux x86_64;
- Python ABI3 wheel — Windows x86_64;
- Python ABI3 wheel — macOS x86_64;
- Python ABI3 wheel — macOS arm64;
- `finkit-0.1.3.crate` source package;
- `finkit-cli-linux-x86_64`;
- `SHA256SUMS`.

Node.js, Java/JNI, and C/C++ are validated by the repository's multi-language CI from source, but their packages are not part of the current GitHub Release asset set. Go, .NET, Android, iOS, and WASM remain source/development integrations rather than v0.1.3 public binary-distribution contracts.

Registry publication is independent from repository packaging. Do not document `pip install finkit`, `cargo add finkit`, `npm install finkit`, Maven Central, NuGet, or `go get` as generally available until the corresponding registry entry has actually been published and verified.

## User documentation

| Document | Purpose |
| --- | --- |
| [installation.md](installation.md) | Supported release assets, prerequisites, source builds, installation verification |
| [usage.md](usage.md) | End-to-end examples and runtime conventions |
| [python.md](python.md) | Python ABI3, NumPy, `CompiledFormula`, pandas, troubleshooting |
| [indicators.md](indicators.md) | Human-readable indicator reference |
| [features.md](features.md) | Feature/capability overview |
| [formula.md](formula.md) | Formula syntax and evaluation |
| [formula/grammar.md](formula/grammar.md) | Formula grammar reference |
| [formula/pine-grammar.md](formula/pine-grammar.md) | Pine grammar/compatibility reference |
| [formula-runtime.md](formula-runtime.md) | Persistent compiled plans and incremental execution |
| [formula-runtime-contract.md](formula-runtime-contract.md) | Ownership, range/last/append, warm-up and concurrency semantics |
| [formula-templates.md](formula-templates.md) | Reusable formula patterns |
| [formula-debugger.md](formula-debugger.md) | Formula debugging workflow |
| [formula-performance.md](formula-performance.md) | Formula optimization and benchmark notes |
| [migration/pine-to-alphata.md](migration/pine-to-alphata.md) | Pine/terminal formula migration guidance |

## API and architecture

| Document | Purpose |
| --- | --- |
| [api-reference.md](api-reference.md) | Public API reference |
| [api-reference-zh.md](api-reference-zh.md) | 中文 API 参考 |
| [core-contracts.md](core-contracts.md) | ComputePlan/FactorPlan/MarketFrame/function registry contracts |
| [function-schema.md](function-schema.md) | Versioned machine-readable function schema |
| [architecture/overview.md](architecture/overview.md) | Crate/binding architecture |
| [architecture/dataflow.md](architecture/dataflow.md) | Batch, streaming, formula, and binding data flow |
| [architecture/formula-engine.md](architecture/formula-engine.md) | Formula parser/compiler/runtime internals |
| [ffi/memory-contract.md](ffi/memory-contract.md) | C ABI ownership/lifetime contract |
| [ffi/error-codes.md](ffi/error-codes.md) | Cross-language error codes |

## Performance and quality

| Document | Purpose |
| --- | --- |
| [benchmark-results.md](benchmark-results.md) | Current benchmark summary |
| [BENCHMARK_VS_TALIB.md](BENCHMARK_VS_TALIB.md) | TA-Lib comparison methodology |
| [BENCHMARK_REPORT.md](BENCHMARK_REPORT.md) | Generated benchmark snapshot |
| [FUZZING.md](FUZZING.md) | Fuzz targets and crash reproduction |
| [development.md](development.md) | Required local and CI checks |

Benchmark numbers are measured snapshots, not universal guarantees. Re-run the benchmark harness on the target CPU/compiler before making latency or throughput commitments.

## Generated source of truth

The following files are generated or machine-readable contracts and should be kept even when simplifying user documentation:

- `indicator_registry.json` — canonical indicator registry snapshot;
- `generated/indicators.md` — generated indicator catalog;
- `generated/streaming-indicators.md` — generated streaming registry;
- `generated/formula-functions.md` — generated formula function list;
- `generated/features.md` — generated feature matrix;
- `generated/error-codes.md` — generated error-code reference;
- `generated/pine-compatibility.md` — generated Pine compatibility matrix;
- `generated/version-matrix.md` — generated release/version matrix;
- `benchmark-baseline.json` — performance-gate baseline where used by CI/scripts.

`scripts/gen_ssot_docs.py --check` and `scripts/check_versions.py` are CI gates for these contracts. `scripts/check_docs_links.py` is also run by Docs Check so repository-local Markdown links fail CI when a referenced file is removed or moved.

## Documentation policy

The documentation tree intentionally does **not** keep completed release plans, obsolete version roadmaps, stale PRD snapshots, duplicate mdBook placeholder pages, or release notes for version lines that never became the current product baseline. Git history and closed pull requests remain the historical record.

When updating docs:

- describe only behavior present in current code;
- distinguish “buildable from source” from “published package”;
- distinguish “CI validated” from “available as a GitHub Release asset”;
- link exact machine-generated catalogs instead of hard-coding counts that can drift;
- keep examples executable against current public APIs;
- update `README.md`, `docs/README.md`, installation/usage docs, and binding READMEs together when a release contract changes.

_Last reviewed against `v0.1.3` and the current main branch: 2026-09-03._
