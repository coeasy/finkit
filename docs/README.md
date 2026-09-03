# Finkit Documentation

This directory is the canonical documentation set for the current Finkit codebase and the published **v0.1.3** baseline. Documentation is organized around tasks users can actually perform today; historical plans and stale release assumptions belong in Git history, not in the active documentation tree.

## Start here

New users should follow this order:

1. [Getting started](getting-started.md) — first successful indicator/formula/CLI run.
2. [Installation](installation.md) — verified release assets, source builds, prerequisites, checksums.
3. [Complete usage guide](usage.md) — data conventions, Python/Rust usage, formulas, streaming, factors/runtime.
4. [CLI guide](cli.md) — file formats, indicator/formula/streaming commands, troubleshooting.
5. [Language bindings](language-bindings.md) — Python, Rust, Node.js, Java/JNI, C/C++, Go, .NET, mobile/WASM status.
6. [Runtime and factors](runtime-and-factors.md) — MarketFrame, factor plans, dependency safety and reuse.

## What is actually distributed in v0.1.3

The GitHub `v0.1.3` Release is the authoritative binary/source distribution for this version. It contains:

- Python ABI3 wheel — Linux x86_64;
- Python ABI3 wheel — Windows x86_64;
- Python ABI3 wheel — macOS x86_64;
- Python ABI3 wheel — macOS arm64;
- `finkit-0.1.3.crate`;
- `finkit-cli-linux-x86_64`;
- `SHA256SUMS`.

Node.js, Java/JNI, and C/C++ build/package paths are verified by multi-language CI from source, but those packages are not claimed as public registry distributions in v0.1.3. Go, .NET, Android, iOS, and WASM remain source/development integrations.

Registry publication is a separate contract. Do not document `pip install finkit`, `cargo add finkit`, `npm install finkit`, Maven Central, NuGet, or public `go get` installation as generally available until that exact package/version is actually published and smoke-tested.

## User guides

| Document | Purpose |
| --- | --- |
| [getting-started.md](getting-started.md) | Fast path from installation to verified calculations |
| [installation.md](installation.md) | Release assets, prerequisites, source builds and installation verification |
| [usage.md](usage.md) | End-to-end usage patterns and data/runtime conventions |
| [python.md](python.md) | ABI3 wheels, NumPy, `CompiledFormula`, pandas and troubleshooting |
| [cli.md](cli.md) | CLI input formats, indicator/formula/streaming commands |
| [language-bindings.md](language-bindings.md) | Binding support matrix and source-build instructions |
| [runtime-and-factors.md](runtime-and-factors.md) | Factor/runtime workflow, dependency validation, reuse and alignment |
| [indicators.md](indicators.md) | Human-readable indicator reference |
| [features.md](features.md) | Feature/capability overview |

## Formula system

| Document | Purpose |
| --- | --- |
| [formula.md](formula.md) | Formula syntax and evaluation reference |
| [formula/grammar.md](formula/grammar.md) | Core formula grammar |
| [formula/pine-grammar.md](formula/pine-grammar.md) | Supported Pine grammar subset |
| [formula-runtime.md](formula-runtime.md) | Persistent compiled plans and incremental execution |
| [formula-runtime-contract.md](formula-runtime-contract.md) | Ownership, `eval_range`, `eval_last`, append, warm-up and concurrency semantics |
| [formula-templates.md](formula-templates.md) | Reusable formula patterns |
| [formula-debugger.md](formula-debugger.md) | Formula debugging workflow |
| [formula-performance.md](formula-performance.md) | Formula optimization and benchmark notes |
| [migration/pine-to-finkit.md](migration/pine-to-finkit.md) | Pine indicator migration guidance and semantic boundaries |

For exact supported functions and Pine mappings, prefer the generated catalogs over hard-coded counts or compatibility percentages.

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

## Generated source of truth — do not delete as “old docs”

The following files are generated or machine-readable contracts and are intentionally retained even when simplifying prose documentation:

- `indicator_registry.json` — canonical indicator registry snapshot;
- `generated/indicators.md` — generated indicator catalog;
- `generated/streaming-indicators.md` — generated streaming registry;
- `generated/formula-functions.md` — generated formula function list;
- `generated/features.md` — generated feature matrix;
- `generated/error-codes.md` — generated error-code reference;
- `generated/pine-compatibility.md` — generated Pine compatibility matrix;
- `generated/version-matrix.md` — generated release/version matrix;
- `benchmark-baseline.json` — performance-gate baseline where used by CI/scripts.

`scripts/gen_ssot_docs.py --check` and `scripts/check_versions.py` validate these contracts. `scripts/check_docs_links.py` makes broken repository-local Markdown links fail Docs Check.

## Documentation cleanup policy

Active `docs/` should not contain:

- completed release plans or old version roadmaps;
- stale PRD/progress snapshots;
- temporary repair notes;
- duplicated directory placeholder READMEs when a canonical guide already exists;
- old package/brand names in filenames that imply a current public API;
- unverified registry-install commands;
- hard-coded capability counts/compatibility percentages that are already generated from SSOT.

Git history and closed pull requests remain the historical record.

## Updating documentation safely

When code or release behavior changes:

1. update the user-facing guide that owns the behavior;
2. update `README.md` and this index when the public installation/support contract changes;
3. update binding-specific docs together with package metadata;
4. regenerate SSOT docs through the generator rather than hand-editing generated files;
5. run link/version/SSOT checks;
6. distinguish **buildable from source**, **CI validated**, **GitHub Release asset**, and **public registry package**.

Recommended validation:

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
cargo fmt --all -- --check
cargo test --workspace --doc --locked
```

_Last reviewed against `v0.1.3` and current `main`: 2026-09-03._
