# Finkit Architecture V4 Migration Execution Status

## Goal

Move all indicator execution paths onto canonical kernels and Unified Runtime without breaking parity, performance gates, or multi-language contracts.

## Round 1 - Kernel migration

- [x] MovingAverage kernel foundation
- [x] Welford statistics kernel foundation
- [x] TrueRange volatility kernel foundation
- [x] Monotonic extrema kernel foundation
- [x] Kernel registry foundation

Next:

- migrate legacy indicator adapters
- add parity tests against legacy implementations
- add TA-Lib comparison gates

## Round 2 - Runtime unification

Targets:

- StateArena
- checkpoint/restore
- DirtyRange propagation
- ExecutionPlan DAG
- shared intermediate state reuse

## Round 3 - Production optimization

Targets:

- Formula DAG compiler
- CSE optimization
- kernel fusion
- compute-many execution
- streaming factor runtime

## Acceptance gates

Every migrated family must satisfy:

1. Numerical parity: canonical kernel == legacy implementation == reference implementation.
2. Performance: no regression in release benchmark gates.
3. Memory: no unnecessary allocation on hot paths.
4. Streaming: incremental update correctness.
5. Documentation: architecture and public behavior updated.
