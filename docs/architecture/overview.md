# AlphaTA — Architecture Overview

This document is the canonical reference for the AlphaTA crate structure,
dependency graph, and main entry points. Diagrams are Mermaid (rendered by
GitHub / GitLab / VS Code).

## Crate graph

```mermaid
graph TD
  user[User code] --> core[alpha-ta-core]
  user --> cli[alpha-ta-cli]
  user --> wasm[alpha-ta-wasm]
  user --> py[AlphaTA-python]
  user --> node[AlphaTA-node]
  user --> go[AlphaTA-go]
  user --> java[AlphaTA-java]
  user --> dotnet[AlphaTA-dotnet]
  user --> c[AlphaTA-c]

  py --> core
  node --> core
  go --> core
  java --> core
  dotnet --> core
  c --> core
  wasm --> core
  cli --> core
  cli --> viz[alpha-ta-visualization]
  viz --> core
```

## Crate roles

| Crate | Role | Output |
|-------|------|--------|
| `alpha-ta-core` | Indicators, math, formula engine, traits | `rlib` (consumed by all others) |
| `alpha-ta-cli` | Command-line interface | `bin` + reusable lib |
| `alpha-ta-wasm` | Browser / Node.js WASM bindings | `wasm-bindgen` output |
| `AlphaTA-python` | PyO3 bindings | `.whl` / sdist |
| `AlphaTA-node` | `napi-rs` bindings | `node-addon-api` binary |
| `AlphaTA-go` | `cgo` bindings | C-shim + Go package |
| `AlphaTA-java` | JNI bindings | `.so` / `.dll` + `.jar` |
| `AlphaTA-dotnet` | P/Invoke bindings | NuGet package |
| `AlphaTA-c` | Pure-C ABI | `.a` / `.so` / `.dll` |
| `alpha-ta-visualization` | Chart rendering (SVG/PNG/HTML) | `rlib` |

## Layering

```
┌──────────────────────────────────────────────────────┐
│ User code (Python / Node / Go / Java / .NET / Rust)  │
└──────────────────────────────────────────────────────┘
                       ▲
┌──────────────────────────────────────────────────────┐
│ FFI bindings (per-language)                          │
└──────────────────────────────────────────────────────┘
                       ▲
┌──────────────────────────────────────────────────────┐
│ Public Rust API (alpha-ta-core)                        │
│  ├─ traits      (Ohlcv, StreamingIndicator, Batch…)  │
│  ├─ indicators  (overlap, momentum, volume, …)      │
│  ├─ formula     (parser, AST, bytecode, JIT, SIMD)   │
│  ├─ streaming   (O(1) per-bar updates)               │
│  ├─ features    (ML pipelines, transformations)     │
│  ├─ math        (moving_avg, simd_kernels, stats)    │
│  └─ patterns    (candlestick, chart)                 │
└──────────────────────────────────────────────────────┘
                       ▲
┌──────────────────────────────────────────────────────┐
│ Math + SIMD kernels                                  │
│  • std / no_std split                                │
│  • AVX2 / FMA / BMI2 on x86-64-v3 (perf-gate)        │
└──────────────────────────────────────────────────────┘
```

## Cross-references

- [Data flow](dataflow.md) — request lifecycle.
- [Formula engine](formula-engine.md) — internal stages of formula eval.
- [api-reference.md](../api-reference.md) — public API surface (English).
- [BENCHMARK_REPORT.md](../BENCHMARK_REPORT.md) — performance baseline.
