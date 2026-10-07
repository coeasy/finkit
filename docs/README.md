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
| 评估竞品与性能优势 | [竞品对比与超越路线](competitive-positioning-zh.md) |
| Install and calculate something | [Getting started](getting-started.md) |
| Verify install/release assets | [Installation](installation.md) |
| Prepare or audit a release | [Release checklist](release-checklist.md) |
| Learn the end-to-end APIs | [Complete usage guide](usage.md) |
| Use Python | [Python guide](python.md) |
| Use the CLI | [CLI guide](cli.md) |
| Understand language/package status | [Language bindings](language-bindings.md) |
| Build factor/runtime workloads | [Runtime and factors](runtime-and-factors.md) |
| Understand research architecture | [Factor research architecture](factor-research-architecture.md) |
| Diagnose failures | [Troubleshooting](troubleshooting.md) |
| 查看当前重构路线与审计记录 | [架构、全链路审计与优化改进方案 V4](FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md) |

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

## Correctness model

Two properties are contracts rather than aspirations:

1. **Warm-up composability** — a rolling indicator's leading NaN warm-up run must not poison downstream composition. `MA(MA(CLOSE,5),9)` and `DEA:=EMA(DIF,9)` produce valid values. For fully-finite input the behavior stays bit-for-bit unchanged.
2. **Multi-path agreement** — tree, bytecode, and compiled-plan execution (plus JIT/SIMD when enabled) must return the same numbers for the same input.

Path agreement alone is not evidence of correctness: two paths that are wrong in the same way still agree. Numerical gates therefore pair agreement with **absolute-property assertions** (exact finite-value counts, identities, non-degeneracy) and **duplicate-implementation cross-checks**. Details in [formula-runtime-contract.md](formula-runtime-contract.md) §3.1–§3.2.

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
| [quant-evaluation.md](quant-evaluation.md) | Shared returns/risk/performance evaluation stack and its per-language surface |

Binding-specific source guides also live with their implementations, including `ffi/go-binding/README.md`, `ffi/dotnet-binding/README.md`, `ffi/android-binding/README.md`, `ffi/ios-binding/README.md`, and `wasm/README.md`.

## Formula system

| Document | Purpose |
| --- | --- |
| [formula.md](formula.md) | Formula syntax, evaluation and debugging guidance |
| [formula/grammar.md](formula/grammar.md) | Core formula grammar |
| [formula/pine-grammar.md](formula/pine-grammar.md) | Supported Pine grammar subset |
| [formula/custom-formula-examples.md](formula/custom-formula-examples.md) | Worked examples of user-defined formulas: composable components, named channels, control flow, parameterised templates, streaming + checkpoints, dialects, `DRAW`, compiled-scan `eval_last` |
| [formula-runtime.md](formula-runtime.md) | Persistent compiled plans and incremental execution |
| [formula-runtime-contract.md](formula-runtime-contract.md) | Ownership, `eval_range`, `eval_last`, append, warm-up composability, multi-path agreement and concurrency semantics |
| [formula-templates.md](formula-templates.md) | Reusable formula patterns |
| [formula-talib-contract.md](formula-talib-contract.md) | The formula runtime / TA-Lib catalog split: what "catalog entry" vs "runtime-registered" vs "numerically verified" each claim |
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
| [competitive-positioning-zh.md](competitive-positioning-zh.md) | 竞品能力矩阵、可证明优势与超越路线 |
| [benchmark-results.md](benchmark-results.md) | Current benchmark/evidence summary |
| [BENCHMARK_VS_TALIB.md](BENCHMARK_VS_TALIB.md) | TA-Lib comparison and reproducibility contract |
| [talib-efficiency-deep-dive-zh.md](talib-efficiency-deep-dive-zh.md) | 效率深度对比：测量方法学、非 ✅ 行分类、负结果目录、仍未解决项 |
| [talib-0.8.0-coverage-audit-2026-09-19.md](talib-0.8.0-coverage-audit-2026-09-19.md) | TA-Lib Python 0.8.0 public-surface gap audit |
| [BENCHMARK_REPORT.md](BENCHMARK_REPORT.md) | Checked-in historical benchmark snapshot |
| [finkit-outperform-talib-architecture-v3.md](finkit-outperform-talib-architecture-v3.md) | **Live spec** — the speedup semantics `scripts/benchmark_talib_arch_v3_gate.py` encodes |
| [finkit-vs-talib-performance-optimization-plan.md](finkit-vs-talib-performance-optimization-plan.md) | **Live spec** — backs `scripts/apply_talib_performance_plan.py` |
| [FUZZING.md](FUZZING.md) | Fuzz targets and crash reproduction |
| [development.md](development.md) | Build, test, benchmark, package and CI workflow |

Benchmark values are measured snapshots, not universal latency/throughput guarantees. Re-run the benchmark harness on the target CPU/compiler/runtime before making production commitments. The scheduled `competitive-benchmark.yml` workflow produces commit-bound TA-Lib evidence; it complements, rather than replaces, correctness and regression gates in normal PR CI.

## Dated analyses and roadmaps — records, not guidance

These are dated snapshots: competitive analyses, roadmap proposals and progress
matrices. They are kept for the record and are **not** descriptions of current
behaviour; where one conflicts with the code, the code wins. They are listed
here because a document nothing links to is unreachable, and an unreachable
document is indistinguishable from one that was deleted.

| Document | Date | What it records |
| --- | --- | --- |
| [competitive-analysis/竞品综合对比总览-2026-09-23.md](competitive-analysis/竞品综合对比总览-2026-09-23.md) | 2026-09-23 | Cross-competitor summary |
| [competitive-analysis/竞品全景图-finkit-2026-09-23.md](competitive-analysis/竞品全景图-finkit-2026-09-23.md) | 2026-09-23 | Competitor landscape map |
| [competitive-analysis/计算库竞品深入分析-finkit-2026-09-23.md](competitive-analysis/计算库竞品深入分析-finkit-2026-09-23.md) | 2026-09-23 | Deep dive: numeric libraries |
| [competitive-analysis/公式系统竞品深入分析-finkit-2026-09-23.md](competitive-analysis/公式系统竞品深入分析-finkit-2026-09-23.md) | 2026-09-23 | Deep dive: formula systems |
| [competitive-analysis/因子研究竞品深入分析-finkit-2026-09-23.md](competitive-analysis/因子研究竞品深入分析-finkit-2026-09-23.md) | 2026-09-23 | Deep dive: factor research |
| [competitive-analysis/finkit-竞争战略与优先级排序-2026-09-23.md](competitive-analysis/finkit-竞争战略与优先级排序-2026-09-23.md) | 2026-09-23 | Strategy and prioritisation |
| [competitive-analysis/finkit-差距分析与战略补足建议书-2026-09-23.md](competitive-analysis/finkit-差距分析与战略补足建议书-2026-09-23.md) | 2026-09-23 | Gap analysis and remediation proposal |
| [competitive-analysis/finkit-全量覆盖与工业级收敛方案-2026-09-23.md](competitive-analysis/finkit-全量覆盖与工业级收敛方案-2026-09-23.md) | 2026-09-23 | Full-coverage convergence proposal |
| [competitive-analysis/finkit-落地开发计划-2026-09-23.md](competitive-analysis/finkit-落地开发计划-2026-09-23.md) | 2026-09-23 | Implementation plan |
| [chan-visualization-roadmap-zh.md](chan-visualization-roadmap-zh.md) | — | 缠论可视化路线图 |
| [chart-improvement-plan-zh.md](chart-improvement-plan-zh.md) | — | 图表能力改进计划 |
| [gpu-rendering-architecture-zh.md](gpu-rendering-architecture-zh.md) | — | GPU 渲染架构设想 |
| [market-calendar-adapters-zh.md](market-calendar-adapters-zh.md) | — | 交易日历适配器方案 |

## Current refactor baseline

**There is exactly one execution baseline:
[`FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`](FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md).**
Everything else below is either a constraint source or an audit trail — not a competing plan.

| Document | Purpose |
| --- | --- |
| [FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md](FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md) | **唯一执行基线** —— 架构/全链路审计、Batch 0–4 落地状态表、§28–§30 历轮审计记录 |
| [refactor-plan-2026-09-21.md](refactor-plan-2026-09-21.md) | **约束来源（非执行基线）** —— 用户已确认的产品边界与定调（不做回测/选股、JIT/`eval_simd` 冻结）与方法论记录 |
| [runtime-carrier-adoption-plan-2026-09-20.md](runtime-carrier-adoption-plan-2026-09-20.md) | R2/R3/R4 声明式载体落地规格（进行中） |

Superseded **architecture/refactor** plans were archived until 2026-10-05 and
are now **deleted from the working tree**; git history keeps every one of them
permanently (see `CHANGELOG.md`, twelfth pass). Keeping a directory whose own
index marked every entry "Superseded" cost a reader more than it saved: the
documents described modules that no longer exist
(`runtime_engine.rs`, the parallel `crates/finkit-*` track) and quoted numbers
that later rounds moved on from. `git log --diff-filter=D -- docs/archive/`
recovers any file by name.

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

_Last product/documentation review: 2026-10-04. Workspace target: v0.2.0. Published distribution baseline: v0.1.15._
