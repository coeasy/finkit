# Finkit Architecture V4 Upgrade Plan

## 1. Vision

Finkit evolves from a high-performance indicator library into a unified quantitative compute runtime.

Target capabilities:

- Indicator computation
- Formula execution
- Factor research
- Streaming analytics
- Multi-language SDKs
- Large-scale quantitative workloads

Core principle:

> One execution engine, multiple research and production workflows.

---

## 2. Current Foundation

Already completed:

- Rust native compute core
- TA-Lib parity framework
- SIMD accelerated kernels
- Unified Runtime foundation
- Typed Artifact identity
- DirtyRange incremental execution
- FactorPlan execution path
- Multi-language bindings
- Reproducible benchmark gates

---

## 3. Architecture V4 Target

```
Data Source
    |
Canonical Data Model
    |
Compiler / Planner
    |
DAG Execution Plan
    |
Kernel Scheduler
    |
BufferArena + StateArena
    |
SIMD / Vector Kernel
    |
Artifact Store
```

---

## 4. Formula Runtime Upgrade

### Goals

Replace repeated indicator execution with compiled execution plans.

### Features

- Expression compilation
- DAG optimization
- Common sub-expression elimination
- Kernel fusion
- Buffer lifetime analysis
- Scratch buffer reuse

Example:

```
RSI(14)
MACD(12,26,9)
FactorA
FactorB
```

Shared computations:

```
EMA
Rolling Mean
TR
ATR
StdDev
```

should execute once.

---

## 5. Streaming Runtime Upgrade

Target:

```
Market Event
    |
Dirty Range Resolver
    |
Incremental Planner
    |
State Arena
    |
Kernel Update
```

Capabilities:

- append_bar incremental execution
- checkpoint
- rollback
- state persistence
- partial replay

---

## 6. Factor Research Platform

Build complete research workflow:

```
Data
 |
Factor Generation
 |
Neutralization
 |
Ranking
 |
Portfolio Construction
 |
Backtest
 |
Attribution
```

Planned support:

- IC
- RankIC
- IC decay
- turnover
- factor exposure
- industry neutralization
- style neutralization
- factor correlation

---

## 7. Canonical Kernel Consolidation

Reduce duplicated implementations.

Target kernels:

- RollingKernel
- EMA Kernel
- WMA Kernel
- Welford Statistics Kernel
- TR/ATR Kernel
- DM Kernel
- Extrema Kernel
- Hilbert Kernel

Indicators become compositions instead of independent implementations.

---

## 8. Performance Goals

Maintain existing contracts:

- TA-Lib numerical parity
- zero-allocation hot paths
- DirtyRange correctness
- reproducible benchmarks

New goals:

- compute-many speedup: 2-5x
- memory reduction: 30-60%
- streaming latency reduction
- large universe factor computation

---

## 9. Competitive Strategy

Finkit should not compete only on indicator count.

Competitive advantages:

| Capability | Strategy |
|---|---|
| Correctness | TA-Lib parity |
| Speed | Rust + SIMD + Kernel fusion |
| Incremental | DirtyRange runtime |
| Research | Factor pipeline |
| Integration | Multi-language SSOT |
| Production | Streaming runtime |

---

## 10. Implementation Order

Phase 1:

- Formula compiler foundation
- DAG optimizer
- Kernel registry

Phase 2:

- BufferArena optimization
- StateArena persistence
- Streaming planner

Phase 3:

- Factor research platform
- Workload benchmarks
- Distributed execution foundation

---

## 11. Release Roadmap

### v0.1.x

Performance hardening and runtime foundations.

### v0.2.0

Architecture V4 milestone:

- DAG runtime
- Formula compiler
- Streaming execution
- Factor research foundation

### Future

Enterprise-scale quantitative compute platform.
