# Finkit Quant Factor Computing Engine V2

## Positioning

Finkit evolves from a technical indicator library into a Rust-native, high-performance, cross-language Quant Factor Computing Engine.

Goals:

- Rust native compute kernel
- Multi-language SDK support
- TA-Lib compatibility
- Factor graph execution
- Streaming incremental computation
- Integration foundation for qianxing

## Architecture

```
Applications
  |
Bindings (Python/Node/WASM/C/Go/Java/.NET)
  |
Factor Runtime
  |
Factor Graph + Execution Plan
  |
Factor Kernel / Indicator Kernel
  |
Math Kernel + SIMD
  |
Core Data Model
```

## Module layout

```
crates/

finkit-core
finkit-array
finkit-series
finkit-math
finkit-factor
finkit-runtime
finkit-stream
finkit-dsl

bindings/
python
node
wasm
c
go
java
dotnet
```

## Core concepts

### QuantSeries

Unified financial time-series representation.

Contains:

- symbol
- timestamps
- values
- metadata

### Factor

Every indicator and feature implements a common factor interface.

### Factor Runtime

Responsible for:

- graph optimization
- dependency reuse
- execution planning
- caching

### Streaming Runtime

Supports incremental updates:

- tick
- bar
- realtime factor calculation

## Migration order

1. Introduce core data contracts
2. Split compute kernels from financial factors
3. Add Factor API
4. Migrate indicators
5. Introduce DAG runtime
6. Add zero-copy language bindings
7. Complete benchmark gates

## Non-goals

Finkit does not provide:

- market data collection
- trading execution
- strategy management

Those belong to higher-level systems such as qianxing.
