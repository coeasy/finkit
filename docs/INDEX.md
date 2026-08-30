# Finkit Documentation Index

> Central navigation hub for the Finkit (`finkit` workspace) docs.
> The single source of truth for indicators is
> [`indicator_registry.json`](./indicator_registry.json), which also drives
> `scripts/gen_ssot_docs.py` (docs) and `scripts/gen_c_header.py` (C FFI header).

## 权威文档（Authoritative）

规划与版本叙事以以下文档为准，其余规划/总结文档视为历史记录（已归档或 superseded）：

- [V1 对齐与修复计划](./plan/FINKIT_V1_ALIGNMENT_AND_FIX_PLAN.md) — 版本路线 v0.1.0 / 命名统一 / P0 修复
- [深度审计与优化方案](./plan/FINKIT_DEEP_AUDIT_AND_OPTIMIZATION_PLAN.md) — 功能点盘点 / 潜在问题 / 断链 / 优化路线
- [Binding 成熟度](./BINDING_TIERS.md) — 各语言绑定发布状态（beta/experimental）
- [兼容性矩阵](./COMPAT_MATRIX.md) / [Pine 兼容性](./PINE_COMPAT_MATRIX.md)

## Getting started
- [README](./README.md) — project overview
- [Development guide](./development.md) — building & testing
- [Installation](./installation.md) — language packages
- [Planning](./PLANNING.md) / [PRD](./PRD.md) / [Progress](./PROGRESS.md)

## Architecture & design
- [Architecture](./architecture/) — module & system design
- [Formula engine](./formula.md) — AST / bytecode / JIT / SIMD
- [Streaming vs TA-Lib efficiency](./STREAMING_VS_TALIB_EFFICIENCY.md)
- [Optimization plan](./OPTIMIZATION_PLAN.md) — perf watch-list (WMA / KAMA / MFI / STOCHF / WILLR / AROON / AD / ADOSC / OBV …)

## Indicators & API
- [Indicators](./indicators.md)
- [API reference](./api-reference.md) / [中文](./api-reference-zh.md)
- [Features](./features.md)
- [Formula engine](./formula.md)
- [Formula debugger](./formula-debugger.md)
- [Development guide](./development.md)
- [Indicator registry (SSOT)](./indicator_registry.json)
- [Generated docs](./generated/) — produced by `scripts/gen_ssot_docs.py`

## FFI & bindings
- [FFI docs](./ffi/)
- [Binding tiers](./BINDING_TIERS.md)
- [Compatibility matrix](./COMPAT_MATRIX.md) / [Pine](./PINE_COMPAT_MATRIX.md)
- C header generator: `scripts/gen_c_header.py` (run `make gen-c-header`; CI runs `make verify-ffi`)

## Benchmarks & quality
- [Benchmark report](./BENCHMARK_REPORT.md) / [vs TA-Lib](./BENCHMARK_VS_TALIB.md)
- [Finkit vs TA-Lib](./ALPHATA_VS_TALIB.md)
- [Fuzzing](./FUZZING.md)

## Migration & agents
- [Migration](./migration/)
- [AGENTS](./AGENTS.md)

## Refactoring
- [Refactoring plan](./../REFACTORING_PLAN.md) — structure cleanup, build consolidation, FFI codegen, golden tests.
