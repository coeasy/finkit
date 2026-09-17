# Finkit

[![CI](https://github.com/coeasy/finkit/actions/workflows/ci.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/ci.yml)
[![Docs Check](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE)

**A Rust-powered quantitative finance compute engine for research, realtime analytics, factor workflows, and multi-language products.**

Finkit is a reusable **Quant Compute Runtime**. It brings technical indicators, formulas, factor DAGs, streaming analytics, feature engineering, research workflows, and language bindings onto one canonical Rust core and one set of numerical/runtime contracts.

Use Finkit behind research notebooks, market scanners, factor platforms, analytics APIs, realtime dashboards, data pipelines, or financial SDKs when you want one calculation layer instead of separate implementations for every product surface.

[中文说明](README.zh-CN.md) · [Product overview](docs/product-overview.md) · [中文产品说明](docs/product-overview-zh.md) · [Competitive strategy](docs/competitive-positioning-zh.md) · [Documentation](docs/README.md)

Current published release: **v0.1.15**. The Unified Runtime and Factor Research work in PR #29 is next-release candidate architecture and is not treated as released until the same commit passes the full release gate.

## What problem does Finkit solve?

Quant systems rarely become difficult because one indicator is missing. They become difficult because the same calculations are reimplemented across notebooks, services, streaming paths, factor platforms, and SDKs. Over time, numerical semantics drift, formulas are parsed repeatedly, dependency graphs are rediscovered, caches diverge, and small data revisions trigger unnecessary full recomputation.

Finkit's product rule is straightforward: **one Rust core, canonical kernels, reusable execution plans, shared correctness contracts, and multiple delivery surfaces.**

## Why teams use Finkit

| Product value | What it means |
| --- | --- |
| Canonical numerical semantics | Different bindings reuse one implementation instead of maintaining algorithm forks |
| Reusable execution plans | Compile and validate formula/factor dependencies once, then execute repeatedly |
| Batch and realtime continuity | Batch evaluation, streaming state, and safe local recomputation share contracts |
| Research-ready primitives | Indicators, transforms, labels, statistics, factor graphs, validation, and research analysis can compose |
| Explainable performance | SIMD, zero-copy, `_into`, streaming, and DirtyRange optimizations have explicit safety boundaries |
| Multi-language delivery | Rust owns the algorithms; bindings expose the product to other runtimes |

## What can you build with it?

**Research platforms.** Calculate indicators, formulas, features, labels, factors, and research statistics with fewer notebook-to-production semantic gaps.

**Market scanners and analytics services.** Precompile reusable formulas and factor graphs and apply them consistently across many aligned datasets.

**Realtime analytics.** Update streaming indicators one bar at a time and avoid unnecessary recomputation where a dependency chain can prove local execution is safe.

**Factor and ML pipelines.** Reuse canonical rolling statistics, normalization, labels, regression, ranking, information metrics, and validation primitives instead of rebuilding the math around each experiment.

**Financial SDKs.** Keep numerical behavior in one Rust engine and expose it through Python, CLI, Node, Java, C/C++, Go, .NET, mobile, or WASM delivery layers.

## Product capability map

| Layer | Capability | Position |
| --- | --- | --- |
| Technical Analysis | Trend, momentum, volatility, volume, cycle, statistics, price transforms, patterns, market helpers | Stable core |
| Formula Engine | Parser, compiler, cache, bytecode/JIT, range/last evaluation, append workflows | Stable core |
| Streaming Engine | Bar-by-bar updates, retained state, batch/stream parity validation | Stable core |
| Feature Engineering | Lags, rolling statistics, normalization, labels, combinations, selection, export | Stable core |
| Factor Engine | Named factor DAGs, dependency validation, borrowed inputs, `FactorPlan` | Stable core / expanding |
| Unified Runtime | Typed artifacts, full/borrowed/range/into execution, DirtyRange | Next-release candidate |
| Factor Research | Prepare, validate, analyze, multi-factor, portfolio, reporting workflows | Next-release expansion |
| Multi-language Delivery | Rust, Python, CLI plus native/mobile/WASM binding paths | Publication state varies |

## One runtime, multiple workloads

```text
Market data / external arrays
          │
          ▼
  MarketFrame / typed inputs
          │
          ├──────── Formula
          ├──────── Factors
          ├──────── Streaming
          └──────── Research
                   │
                   ▼
            ComputePlan / FactorPlan
                   │
                   ▼
             Unified Runtime
        full / borrowed / range / into
                   │
         ┌─────────┴─────────┐
         ▼                   ▼
    retained state       typed artifacts
```

The next-release runtime candidate adds typed deterministic `ArtifactHash`, routes `FactorPlan` through the shared `UnifiedRuntime`, and turns `DirtyRange` into an actual local execution boundary. Local recomputation is allowed only when the complete dependency chain proves it is incremental, fixed-lookback, and time-series-safe. Cross-sectional, dynamic-lookback, or unknown contracts fall back to full execution.

That policy is intentional: **correct first, fast second.**

## Quick start: Python

The authoritative v0.1.15 binary distribution is the GitHub Release. Download the wheel matching your platform and install it locally:

```bash
python -m pip install ./finkit-0.1.15-<platform>.whl
```

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)

sma20 = ta.sma(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, 12, 26, 9)

print(sma20[-1], rsi14[-1], macd[-1])
```

## Quick start: Rust

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
    println!("{:?} {:?}", sma20.last(), rsi14.last());
    Ok(())
}
```

## Performance and correctness

Finkit optimizes the whole execution path: SIMD kernels, borrowed/zero-copy inputs, caller-owned `_into` outputs, persistent compiled plans, streaming state, dependency reuse, local DirtyRange recomputation, and reusable buffers.

Those optimizations remain constrained by numerical correctness. Time series are oldest-to-newest, aligned arrays stay aligned, rolling outputs preserve warm-up `NaN`, predictive research follows point-in-time/no-lookahead rules, and unsafe incremental plans must fall back to full execution.

Strict performance contracts run in dedicated release-mode regression gates. The HT_SINE whole-function budget remains `<1000 ns/bar`, but is measured in an optimized single-threaded gate instead of a noisy concurrent debug test. DirtyRange also carries a row-efficiency contract: a local historical revision must remain local and must match a full recomputation exactly.

## Competitive performance is evidence-based

Finkit does not treat “fastest” as a permanent adjective. Direct competitor results are tied to a commit, CPU/platform, compiler, dataset, feature set, and competitor version.

The repository includes a reproducible TA-Lib C comparison pipeline, machine-readable reports, and a scheduled TA-Lib 0.7.1 head-to-head guardrail. The long-term target for canonical hot paths is to match or beat TA-Lib while preserving parity; the broader differentiation is reusable Formula/Factor plans, safe DirtyRange recomputation, low-allocation execution, research reuse, and one semantic core across language bindings.

See [the competitive positioning and superiority roadmap](docs/competitive-positioning-zh.md) and [TA-Lib benchmark contract](docs/BENCHMARK_VS_TALIB.md).

## Distribution status is explicit

Finkit distinguishes source availability, CI validation, package candidates, GitHub Release assets, and public registry packages. A binding existing in the repository does not automatically mean that npm, Maven, NuGet, Go, Android, Swift, or another registry package is publicly released.

See [docs/language-bindings.md](docs/language-bindings.md) for the exact support/publication matrix.

## Documentation

- [Product overview](docs/product-overview.md)
- [中文产品说明](docs/product-overview-zh.md)
- [中文宣传文稿](docs/promotion-zh.md)
- [品牌与媒体素材](docs/media-kit-zh.md)
- [竞品对比与超越路线](docs/competitive-positioning-zh.md)
- [Benchmark evidence](docs/benchmark-results.md)
- [Getting started](docs/getting-started.md)
- [Complete usage guide](docs/usage.md)
- [Runtime and factors](docs/runtime-and-factors.md)
- [Factor research architecture](docs/factor-research-architecture.md)
- [Language bindings](docs/language-bindings.md)
- [Development and verification](docs/development.md)

## What Finkit is not

Finkit is not an OMS, brokerage adapter, exchange gateway, matching engine, or turnkey live-trading platform. It is the **calculation, research, and realtime analytics engine** those systems can call.

## License

Finkit is dual-licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
