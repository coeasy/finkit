# Finkit Documentation

Welcome to the Finkit (Finkit) documentation index. Finkit is a high-performance financial technical analysis library written in Rust with multi-language bindings.

> **mdBook site:** Build locally with `mdbook build docs` (requires [mdBook](https://rust-lang.github.io/mdBook/)). Navigation lives in [`src/SUMMARY.md`](src/SUMMARY.md).

---

## Getting Started

| Document | Description |
|----------|-------------|
| [Installation Guide](installation.md) | Detailed per-language installation (pip, npm, Maven, etc.) |
| [Development Guide](development.md) | Build from source, testing, and dev workflow |

---

## User Guide

### Indicators & Features

| Document | Description |
|----------|-------------|
| [Indicator List](indicators.md) | Complete catalog of batch and streaming indicators |
| [Features Overview](features.md) | Capability matrix across languages and platforms |

### Formula Engine

| Document | Description |
|----------|-------------|
| [Formula Engine](formula.md) | Expression-based computation (`MA(CLOSE, 20)`) |
| [Formula Templates](formula-templates.md) | Pre-built formula patterns |
| [Formula Performance](formula-performance.md) | Benchmarks and optimization notes |
| [Formula Debugger](formula-debugger.md) | Debugging formula expressions |
| [Architecture: Formula Engine](architecture/formula-engine.md) | Internal design of the formula subsystem |

### Streaming & Batch

| Document | Description |
|----------|-------------|
| [Architecture: Dataflow](architecture/dataflow.md) | Batch vs streaming data paths |
| [Architecture: Overview](architecture/overview.md) | System architecture overview |

---

## API Reference

| Document | Description |
|----------|-------------|
| [API Reference (English)](api-reference.md) | Complete API for Rust core and all language bindings |
| [API Reference (中文)](api-reference-zh.md) | 技术指标函数详细参考（中文） |

---

## Benchmarks

| Document | Description |
|----------|-------------|
| [Benchmark vs TA-Lib](BENCHMARK_VS_TALIB.md) | How to read `bench-vs-talib` output |
| [Benchmark Report](BENCHMARK_REPORT.md) | Detailed benchmark report |

---

## Migration Guide

| Document | Description |
|----------|-------------|
| [Changelog](../CHANGELOG.md) | Version history and release notes |

---

## FFI & Bindings

| Document | Description |
|----------|-------------|
| [FFI Memory Contract](ffi/memory-contract.md) | Memory ownership and lifetime rules across FFI |
| [FFI Error Codes](ffi/error-codes.md) | Error code reference for C/FFI consumers |
| [Python Binding](../ffi/python-binding/README.md) | Python package usage |
| [Node.js Binding](../ffi/node-binding/README.md) | Node.js / npm package usage |
| [Java Binding](../ffi/java-binding/README.md) | Java / Maven integration |
| [Go Binding](../ffi/go-binding/README.md) | Go module usage |
| [.NET Binding](../ffi/dotnet-binding/README.md) | .NET / NuGet integration |
| [Android Binding](../ffi/android-binding/README.md) | Android JNI integration |
| [iOS Binding](../ffi/ios-binding/README.md) | iOS framework integration |
| [Packaging Usage](../packaging/usage/README.md) | Cross-language packaging examples |

---

## Contributing

| Document | Description |
|----------|-------------|
| [Contributing](../CONTRIBUTING.md) | Contribution guidelines (root) |
| [Development Guide](development.md) | Build from source, testing, and dev workflow |
| [Fuzzing](FUZZING.md) | Fuzz testing setup and targets |

---

## [Internal] Project Artifacts

These files support autonomous development and project planning. They are **not** intended for end-user consumption.

| Document | Description |
|----------|-------------|
| [AGENTS.md](AGENTS.md) | [Internal] AzaLoop agent configuration and workflow |
| [PLANNING.md](PLANNING.md) | [Internal] Implementation planning notes |
| [PROGRESS.md](PROGRESS.md) | [Internal] Story completion tracker |
| [PRD.md](PRD.md) | [Internal] Product requirements document |
| [Compatibility Matrix](COMPAT_MATRIX.md) | [Internal] TA-Lib compatibility matrix |
| [Pine Compatibility Matrix](PINE_COMPAT_MATRIX.md) | [Internal] Pine Script compatibility matrix |
| [Binding Tiers](BINDING_TIERS.md) | [Internal] FFI binding tier classification |

---

_Last updated: 2026-06-25. Archived documents (LEARNINGS, RELEASE_NOTES, old optimization plans) have been removed to reduce clutter._
