# Finkit

[![CI](https://github.com/coeasy/finkit/actions/workflows/ci.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/ci.yml)
[![Docs Check](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE)

**A Rust-powered quantitative finance engine for indicators, formulas, factors, streaming analytics, and research workflows.**

Finkit turns financial computation into reusable infrastructure. Instead of maintaining separate implementations for batch indicators, terminal-style formulas, factor graphs, streaming calculations, and language bindings, Finkit builds them around one Rust core and one set of numerical/runtime contracts.

Use it when you need a fast calculation engine behind notebooks, research services, screening systems, analytics APIs, dashboards, or multi-language SDKs — without coupling the calculation layer to brokerage, order management, or a specific trading platform.

[中文说明](README.zh-CN.md) · [Product overview](docs/product-overview.md) · [Documentation](docs/README.md) · [Current release](https://github.com/coeasy/finkit/releases/tag/v0.1.15)

Current published release: **v0.1.15**.

## Why Finkit

### One calculation core

Rust owns the canonical numerical implementation. Python, Node.js, Java/JNI, C/C++, Go, .NET, Android, iOS, WASM, CLI, and other adapters are delivery surfaces rather than independent algorithm forks.

### Reusable execution instead of repeated parsing

Compile formulas and factor dependency graphs once, validate them once, and reuse them across repeated requests. The runtime is moving toward a shared execution model for batch, factor, research, range, and streaming workloads.

### Research-ready, not just indicator-ready

Finkit includes technical indicators, formulas, transforms, feature engineering, labeling, factor graphs, statistics, risk helpers, validation primitives, and the developing Factor Research layer. The goal is to connect the path from raw market series to reusable research artifacts without rebuilding the same math in multiple modules.

### Performance with correctness gates

Hot paths are backed by SIMD, zero-copy/borrowed inputs where appropriate, reusable output buffers, streaming state, and benchmark gates. Numerical behavior is guarded by parity/reference tests, no-lookahead rules for predictive research, SSOT-generated metadata, Clippy, docs checks, and multi-language packaging tests.

## Product capability map

| Layer | What it provides | Status |
| --- | --- | --- |
| Technical analysis | Trend, momentum, volatility, volume, cycle, statistics, price transforms, patterns, A-share helpers | Stable core |
| Formula engine | Parsing, compilation, reusable plans, bytecode/JIT paths, range/last evaluation, append-oriented workflows | Stable core |
| Streaming engine | One-bar-at-a-time indicators, retained state, convergence/parity tests | Stable core |
| Feature engineering | Lags, rolling statistics, normalization, labels, combinations, selection, export | Stable core |
| Factor engine | Named factor DAGs, dependency validation, borrowed inputs, compiled `FactorPlan` | Stable core / expanding |
| Unified Runtime | Shared execution boundary, typed artifact identity, range execution contracts | Next-release architecture |
| Factor Research | Preparation, validation, Alphalens-style analysis, multi-factor, portfolio/risk/report workflows | Next-release expansion |
| Multi-language SDKs | Python, Rust, CLI plus source/CI paths for Node, Java, C/C++, Go, .NET, Android, iOS, WASM | Mixed publication state; see docs |

“CI validated” and “published package” are intentionally different states. See [language bindings](docs/language-bindings.md) before assuming a public registry package exists for a target.

## Architecture

```text
Market data / external arrays
          │
          ▼
  MarketFrame / typed inputs
          │
          ├───────────────┬────────────────┬─────────────────┐
          ▼               ▼                ▼                 ▼
     Indicators        Formula          Factors          Streaming
          │               │                │                 │
          └───────────────┴────────┬───────┴─────────────────┘
                                  ▼
                         ComputePlan / FactorPlan
                                  │
                                  ▼
                           Unified Runtime
                    full / borrowed / range / into
                                  │
                      ┌───────────┴───────────┐
                      ▼                       ▼
               Feature / Research      Runtime artifacts
                      │                       │
                      └───────────┬───────────┘
                                  ▼
             Rust / Python / CLI / native bindings / WASM
```

The architectural rule is simple: **reuse canonical kernels and canonical plans; do not create another hidden copy of the same formula, statistic, dependency graph, or cache.**

## What the next-release runtime adds

The current factor-research branch is consolidating several previously separate concepts into the shared runtime:

- typed, deterministic `ArtifactHash` identity instead of untyped ad-hoc hashes;
- `FactorPlan` execution through the shared `UnifiedRuntime` boundary;
- `DirtyRange` propagation that distinguishes changed input rows, affected output rows, and the historical recomputation window;
- local recomputation only when the complete dependency chain explicitly proves it is incremental, time-series-safe, and fixed-lookback;
- conservative full fallback for cross-sectional, dynamic-lookback, or otherwise unsafe nodes;
- retained output materializations for range/into execution instead of recomputing clean rows.

These are correctness constraints first and performance optimizations second. Finkit will not label an execution path “incremental” when doing so can change results.

## Typical use cases

- **Research notebooks and services** — calculate indicators, formulas, features, labels, factors, and evaluation inputs from the same core.
- **Market scanners and screeners** — precompile reusable formulas/factors and evaluate many aligned datasets consistently.
- **Realtime analytics** — use streaming indicators and retained state for low-overhead bar-by-bar updates.
- **Quant data pipelines** — generate feature matrices, transforms, labels, and research artifacts without duplicating statistical kernels.
- **Analytics APIs** — place Rust computation behind Python, Node, Java, C/C++, Go, .NET, mobile, or WASM delivery layers.
- **Cross-platform SDKs** — keep numerical semantics in one core while exposing idiomatic APIs to multiple runtimes.

## Quick start: Python

The authoritative v0.1.15 binary distribution is the GitHub Release. Download the wheel matching your platform and install it locally:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.15-<platform>.whl
```

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)

sma20 = ta.sma(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(
    close,
    fastperiod=12,
    slowperiod=26,
    signalperiod=9,
)

print("SMA20", sma20[-1])
print("RSI14", rsi14[-1])
print("MACD", macd[-1], signal[-1], hist[-1])
```

Time-series outputs preserve input alignment. Rolling indicators normally contain leading warm-up `NaN` values until enough bars are available.

## Reusable formula execution

```python
import numpy as np
import finkit as ta

n = 1000
open_ = np.arange(n, dtype=np.float64)
high = open_ + 1.0
low = open_ - 1.0
close = open_ + 0.5
volume = np.full(n, 1000.0, dtype=np.float64)

plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

Reusable plans also expose optimized paths such as zero-copy evaluation, range evaluation, latest-value evaluation, and append-oriented execution where supported by the binding/runtime contract.

## Quick start: Rust

Until a public crates.io package/version is independently verified, use the release tag or a local path:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.15" }
```

```rust
use finkit::indicators;
use finkit::math::moving_avg;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let close: Vec<f64> = (1..=100).map(|v| v as f64).collect();
    let sma20 = moving_avg::sma(&close, 20)?;
    let rsi14 = indicators::rsi(&close, 14)?;

    println!("SMA20 = {:?}", sma20.last());
    println!("RSI14 = {:?}", rsi14.last());
    Ok(())
}
```

## Quick start: CLI

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

```bash
./target/release/finkit-cli sma --input close.txt --period 20
./target/release/finkit-cli rsi --input close.txt --period 14 --format json
./target/release/finkit-cli atr --input ohlcv.csv --period 14
./target/release/finkit-cli formula "MA(CLOSE, 5)" --input ohlcv.csv
./target/release/finkit-cli streaming ema --input ohlcv.csv --period 20
```

## Distribution and language status

The GitHub `v0.1.15` Release is the authoritative distribution for that version. It contains verified Python ABI3 wheels, the Rust crate release asset, a Linux x86_64 CLI binary, and checksums.

Other language integrations have different maturity/publication states. Node.js, Java/JNI, C/C++, Go, .NET, Android, iOS, and WASM all have source and/or CI packaging paths, but a build candidate is not automatically a public npm/Maven/NuGet/Go/Android/Swift package.

See [docs/language-bindings.md](docs/language-bindings.md) for the exact contract.

## Correctness and data conventions

Across bindings and runtimes:

- bars are ordered oldest → newest;
- related OHLCV arrays must remain aligned;
- rolling outputs preserve alignment with leading warm-up `NaN` values;
- combine multiple outputs with a joint finite-value mask instead of dropping rows independently;
- zero-copy borrowed inputs must not be resized or mutated concurrently during evaluation;
- predictive research must preserve point-in-time/no-lookahead semantics;
- unsafe incremental plans must fall back to full execution rather than silently changing results.

## Performance model

Finkit optimizes the entire execution path, not only individual formulas:

- SIMD kernels for selected hot paths;
- reusable `_into` output APIs to reduce allocation;
- zero-copy/borrowed input paths where ownership permits;
- persistent compiled formula plans;
- streaming state for bar-by-bar calculations;
- shared factor dependency plans;
- DirtyRange-based local execution for proven-safe dependency chains;
- benchmark and relative-performance gates in CI.

Performance is CPU-, compiler-, feature-, and workload-dependent. Treat checked-in benchmark reports as measured snapshots, not universal latency guarantees.

## Documentation

| Document | Purpose |
| --- | --- |
| [Product overview](docs/product-overview.md) | Product positioning, architecture, capabilities, users, scenarios, roadmap |
| [中文产品说明](docs/product-overview-zh.md) | 中文产品定位与能力说明 |
| [中文宣传文稿](docs/promotion-zh.md) | Website/community/release promotional copy |
| [Getting started](docs/getting-started.md) | First successful installation and calculation |
| [Installation](docs/installation.md) | Release assets, source builds and verification |
| [Complete usage guide](docs/usage.md) | End-to-end usage and runtime conventions |
| [Language bindings](docs/language-bindings.md) | Exact support/publication matrix |
| [Runtime and factors](docs/runtime-and-factors.md) | Unified Runtime, FactorPlan, DirtyRange and factor execution |
| [Factor research architecture](docs/factor-research-architecture.md) | Reuse-first research architecture and implementation sequence |
| [Formula engine](docs/formula.md) | Formula syntax and execution |
| [API reference](docs/api-reference.md) | Public API overview |
| [Development](docs/development.md) | Build, test, benchmark, package and CI workflow |

Generated files under `docs/generated/`, `docs/indicator_registry.json`, and benchmark baselines are machine-readable/CI contracts and are intentionally retained.

## Build and verify

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
```

## What Finkit is not

Finkit is not an OMS, brokerage adapter, exchange gateway, matching engine, or a turnkey live-trading platform. It is the **calculation and research engine** that those systems can call.

Keeping that boundary explicit lets the project optimize numerical correctness, runtime reuse, data alignment, research safety, and cross-language delivery without coupling the core to a broker or execution venue.

## License

Finkit is dual-licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
