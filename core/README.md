# Finkit Core

`core/` is the Rust implementation behind Finkit. It contains the batch and streaming indicator kernels, formula compiler/runtime, factor and market-runtime contracts, transforms, patterns, feature infrastructure, registries, and the public Rust API used by the language bindings.

Current repository release line: **0.1.3**.

## Use the Rust core

Until a crates.io publication is independently verified, use the Git tag or a local checkout rather than documenting `cargo add finkit` as guaranteed availability.

### Git tag

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
```

### Local path

```toml
[dependencies]
finkit = { path = "../finkit/core" }
```

The GitHub `v0.1.3` Release also contains `finkit-0.1.3.crate` as a package artifact.

## Basic example

```rust
use finkit::indicators;
use finkit::math::moving_avg;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let close: Vec<f64> = (1..=100).map(|x| x as f64).collect();

    let sma20 = moving_avg::sma(&close, 20)?;
    let rsi14 = indicators::rsi(&close, 14)?;

    println!("SMA20 last = {:?}", sma20.last());
    println!("RSI14 last = {:?}", rsi14.last());
    Ok(())
}
```

Rolling calculations generally preserve input alignment and use leading `NaN` values during their lookback/warm-up region. Treat those values as part of the indicator contract rather than as an execution failure.

## Main public modules

The core crate exposes the following capability areas:

| Module | Purpose |
| --- | --- |
| `indicators` | Batch technical indicators and indicator families |
| `streaming` | Incremental one-bar-at-a-time calculations |
| `formula` | Formula parser, optimizer, bytecode/JIT/runtime, templates and compatibility |
| `compute` | Unified compute/factor plans and execution policies |
| `factors` | Dependency-aware factor definitions/evaluation |
| `runtime` | Aligned market data/runtime contracts |
| `registry` | Canonical function/indicator registry infrastructure |
| `schema` | Machine-readable function schema support |
| `transforms` | Time-series transforms and pipelines |
| `features` | Feature engineering infrastructure when enabled |
| `patterns` | Candlestick and chart-pattern detection |
| `risk` | Risk/statistical helpers |
| `selectors` | Selection helpers |
| `polars_ext` | Optional Polars integration |

The exact indicator/function inventory is generated from the current registry. Use [`../docs/generated/indicators.md`](../docs/generated/indicators.md) and [`../docs/indicator_registry.json`](../docs/indicator_registry.json) rather than copying a hard-coded count into integrations.

## Feature flags

The default feature set is intended to provide the normal full Finkit experience. It includes the standard library, formula support, serialization/observability scaffolding, formula JIT/SIMD paths, and all indicator categories configured by `core/Cargo.toml`.

Important optional features include:

- `rayon` — parallel execution paths where implemented;
- `finkit-polars` — Polars integration;
- `talib-c` — TA-Lib C comparison/integration support where configured;
- `nightly-avx512` — AVX-512-specific paths requiring the appropriate Rust/toolchain/CPU conditions;
- `precision-f32` — f32-oriented support where implemented;
- profiling/observability-related feature switches defined in `core/Cargo.toml`.

For minimal builds, inspect the current feature graph before disabling defaults because indicator families have transitive feature dependencies.

Example:

```toml
[dependencies]
finkit = {
  git = "https://github.com/coeasy/finkit",
  tag = "v0.1.3",
  default-features = false,
  features = ["std", "indicators-overlap"]
}
```

## Formula runtime

The formula subsystem supports parsing, optimization, bytecode/runtime execution, reusable compiled plans, range/latest-value evaluation, common-subexpression handling, and terminal-compatibility layers.

For application-facing semantics, see:

- [`../docs/formula.md`](../docs/formula.md)
- [`../docs/formula-runtime.md`](../docs/formula-runtime.md)
- [`../docs/formula-runtime-contract.md`](../docs/formula-runtime-contract.md)
- [`../docs/formula/grammar.md`](../docs/formula/grammar.md)

The Python `CompiledFormula` wrapper is built on these core capabilities and exposes reusable `eval`, `eval_zero_copy`, `eval_range`, `eval_last`, `append_bar`, `reserve_bars`, and `reset` workflows.

## Factor and market runtime

Finkit's factor/runtime layer is designed around aligned market data and explicit dependency validation:

1. construct an aligned `MarketFrame`;
2. register/resolve factors;
3. build a dependency-aware factor plan;
4. reject invalid dependencies or cycles;
5. execute the plan against the aligned frame;
6. consume aligned results while respecting warm-up/NaN contracts.

The stable high-level contracts are documented in [`../docs/core-contracts.md`](../docs/core-contracts.md).

## Streaming

Streaming indicators maintain state between bars and avoid recomputing the entire input history when an incremental implementation is available.

General rules:

- push bars in chronological order;
- keep one stateful instance per independent time series/instrument unless explicitly resetting it;
- expect warm-up before a finite value is available;
- do not assume every batch indicator has a streaming counterpart;
- use the generated [`../docs/generated/streaming-indicators.md`](../docs/generated/streaming-indicators.md) as the current support registry.

## Registry and schema

Finkit maintains machine-readable metadata for bindings, tooling, generated docs, and compatibility checks. Relevant artifacts include:

- `docs/indicator_registry.json`;
- `docs/generated/indicators.md`;
- `docs/generated/streaming-indicators.md`;
- `docs/generated/formula-functions.md`;
- `docs/generated/version-matrix.md`;
- the `finkit-schema` CLI built from the workspace CLI package.

Generated files are checked by CI and should be regenerated from source rather than hand-edited to hide drift.

## Build and test

From the repository root:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

Build only the core package:

```bash
cargo build -p finkit --release --locked
```

Create the source package artifact:

```bash
cargo package -p finkit --locked --no-verify
```

## Benchmarks

The repository contains Criterion/custom benchmark suites and CI performance gates. A benchmark result is meaningful only for the measured CPU, compiler, feature set, data shape, and workload.

Examples:

```bash
cargo bench -p finkit --no-run
cargo bench -p finkit
```

See:

- [`../docs/benchmark-results.md`](../docs/benchmark-results.md)
- [`../docs/BENCHMARK_VS_TALIB.md`](../docs/BENCHMARK_VS_TALIB.md)
- [`../docs/BENCHMARK_REPORT.md`](../docs/BENCHMARK_REPORT.md)

Do not turn a historical benchmark snapshot into a universal performance guarantee.

## Language bindings

The Rust core is consumed by the repository's native bindings under `ffi/`.

Current v0.1.3 distribution status is intentionally narrower than source support:

- Python wheels are published as GitHub Release assets;
- Rust `.crate` and Linux x86_64 CLI artifacts are published in the GitHub Release;
- Node.js, Java/JNI, and C/C++ packaging paths are validated in CI from source;
- Go, .NET, Android, iOS, and WASM remain source/development integrations for this release.

See [`../docs/installation.md`](../docs/installation.md) and [`../docs/usage.md`](../docs/usage.md) for the authoritative user-facing status and examples.

## License

Finkit is dual-licensed under MIT OR Apache-2.0. See the repository root license files.
