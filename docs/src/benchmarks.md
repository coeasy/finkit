# Benchmarks

Performance benchmarks for Finkit covering native batch indicators, streaming computation, the formula engine, and SIMD operations. All benchmarks use Criterion.rs with the release profile and can be reproduced locally.

See `docs/benchmarks.md` for methodology and `docs/benchmark-results.md` for latest numbers. Finkit core indicators are typically 1.3x–3.2x faster than TA-Lib C in real FFI benchmarks.
