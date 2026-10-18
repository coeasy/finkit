# Finkit — Architecture Overview

This document is the canonical reference for the Finkit crate structure,
dependency graph, and main entry points. Diagrams are Mermaid (rendered by
GitHub / GitLab / VS Code).

## Crate graph

```mermaid
graph TD
  user[User code] --> core[finkit]
  user --> cli[finkit-cli]
  user --> factor[finkit-factor-analysis]
  user --> viz[finkit-visualization]
  user --> wasm[finkit-wasm]
  user --> py[finkit-python]
  user --> node[finkit-node]
  user --> go[finkit-go]
  user --> java[finkit-java]
  user --> dotnet[finkit-dotnet]
  user --> c[finkit-ffi]
  user --> ios[finkit-ios]
  user --> android[finkit-android]

  factor --> core
  viz --> core
  cli --> factor
  cli --> viz
  cli --> core
  wasm --> ffi_common[finkit-ffi-common]
  wasm --> viz
  wasm --> core
  c --> ffi_common
  c --> core
  go --> ffi_common
  go --> core
  java --> ffi_common
  java --> viz
  java --> core
  dotnet --> ffi_common
  dotnet --> core
  ios --> ffi_common
  ios --> core
  android --> ffi_common
  android --> core
  py --> factor
  py --> viz
  py --> ffi_common
  py --> core
  node --> viz
  node --> ffi_common
  node --> core
  ffi_common --> factor
  ffi_common --> core
```

## Crate roles

| Crate | Role | Output |
|-------|------|--------|
| `finkit` | Indicators, math, formula engine, traits | `rlib` (consumed by all others) |
| `finkit-factor-analysis` | Factor study, metrics, risk and cross-sectional analysis | `rlib` |
| `finkit-visualization` | Chart rendering (SVG/PNG/HTML) | `rlib` |
| `finkit-cli` | Formula/indicator/runtime and factor-study command-line workflows | CLI binaries |
| `finkit-wasm` | Browser / Node.js WASM bindings | `wasm-bindgen` output |
| `finkit-ffi-common` | Shared serialization, request types and language-neutral FFI helpers | `rlib` |
| `finkit-python` | PyO3 bindings | `.whl` / sdist |
| `finkit-node` | `napi-rs` bindings | Node native addon |
| `finkit-go` | `cgo` bindings | C-shim + Go package |
| `finkit-java` | JNI bindings | `.so` / `.dll` + `.jar` |
| `finkit-dotnet` | P/Invoke bindings | NuGet package candidate |
| `finkit-ffi` | C ABI | `.a` / `.so` / `.dll` |
| `finkit-ios` | Swift/iOS wrapper over C ABI | Swift package / XCFramework candidate |
| `finkit-android` | Kotlin/JNI wrapper over C ABI | AAR candidate |

## Layering

```
┌──────────────────────────────────────────────────────┐
│ User code (Python / Node / Go / Java / .NET / Rust)  │
└──────────────────────────────────────────────────────┘
                       ▲
┌──────────────────────────────────────────────────────┐
│ CLI / WASM / Python / Node / C ABI / mobile wrappers │
└──────────────────────────────────────────────────────┘
                       ▲
┌──────────────────────────────────────────────────────┐
│ Core (finkit) + Factor Analysis + Visualization     │
│  ├─ core: indicators / formula / streaming / math   │
│  ├─ factor-analysis: studies / risk / cross-section │
│  └─ visualization: renderers and chart models       │
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
- [Core workflow matrix](workflow-matrix.md) — end-to-end capability and validation coverage.
- [Formula engine](formula-engine.md) — internal stages of formula eval.
- [api-reference.md](../api-reference.md) — public API surface (English).
- [Benchmark results](../benchmark-results.md) — current performance summary.
