# AlphaTA Performance Baseline — vs TA-Lib Comparison

> **Date:** 2026-05-27 | **Environment:** Windows 10 (22621), x86_64 AVX2, Rust 2021 edition
> **Build profile:** `--release` (optimized) via Criterion.rs
> **Benchmark:** `cargo bench -p alpha-ta-core --bench talib_comparison_bench`

## Executive Summary

| Metric | AlphaTA Current | TA-Lib C (reference) | Gap |
|--------|------------|---------------------|-----|
| SMA (500K) | 11.55 ns/val | 0.73 ns/val | 15.8x slower |
| EMA (500K) | 4.04 ns/val | 1.42 ns/val | 2.8x slower |
| Best streaming (EMA 500K) | 3.47 ns/val | — | competitive |

**Key finding:** Native batch path has significant optimization potential (SMA O(n*period) vs O(n)), formula engine adds 1.3x-3.5x overhead, streaming path is already competitive on EMA.

---

## 1. Native Batch Indicators

| Indicator | 10K (ns/iter) | 100K (ns/iter) | 500K (ns/iter) | ns/value (500K) |
|-----------|--------------|----------------|----------------|-----------------|
| SMA(20) | 94,580 | 976,153 | 5,774,027 | **11.55** |
| EMA(12) | 22,940 | 244,780 | 2,018,602 | **4.04** |
| RSI(14) | 56,120 | 565,686 | 3,744,015 | **7.49** |
| MACD(12,26,9) | 106,508 | 1,184,987 | 12,072,840 | **24.15** |
| BOLL(20,2,2) | 440,342 | 4,957,327 | 29,249,375 | **58.50** |
| ATR(14) | 50,164 | 733,354 | 6,613,131 | **13.23** |

### vs TA-Lib C Reference (500K data points, from QuanTAlib benchmarks)

| Indicator | AlphaTA ns/val | TA-Lib C ns/val | Ratio | Status |
|-----------|-----------|-----------------|-------|--------|
| SMA(20) | 11.55 | 0.73 | 15.8x slower | needs optimization |
| EMA(12) | 4.04 | 1.42 | 2.8x slower | needs optimization |
| RSI(14) | 7.49 | ~2.0 (est.) | 3.7x slower | needs optimization |
| MACD | 24.15 | ~4.0 (est.) | 6.0x slower | needs optimization |
| BOLL | 58.50 | ~6.0 (est.) | 9.8x slower | needs optimization |
| ATR | 13.23 | ~2.0 (est.) | 6.6x slower | needs optimization |

---

## 2. Formula Engine (Batch via FormulaEngine)

| Indicator | 10K (ns/iter) | 100K (ns/iter) | Overhead vs native (10K) |
|-----------|--------------|----------------|--------------------------|
| SMA(20) | 124,684 | 1,563,190 | **1.32x** |
| EMA(12) | 61,022 | 715,910 | **2.66x** |
| RSI(14) | 90,130 | 957,200 | **1.61x** |
| MACD | 112,641 | 1,463,390 | **1.06x** |
| BOLL | 535,986 | 4,338,600 | **1.22x** |
| ATR | 173,565 | 1,497,550 | **3.46x** |

**Analysis:** Formula engine overhead varies significantly. MACD and BOLL have good overhead ratios because their native paths are already slow (masking VM overhead). EMA and ATR have high overhead ratios (2.6x-3.5x) because the native path is fast, making VM dispatch/allocation costs dominant.

---

## 3. Streaming Indicators (O(1) per bar)

| Indicator | 10K (ns/iter) | 100K (ns/iter) | 500K (ns/iter) | ns/value (500K) |
|-----------|--------------|----------------|----------------|-----------------|
| SMA(20) | 64,943 | 733,267 | 3,612,663 | **7.23** |
| EMA(12) | 29,595 | 309,313 | 1,735,565 | **3.47** |
| RSI(14) | 71,990 | 880,683 | 5,679,438 | **11.36** |
| MACD(12,26,9) | 70,140 | 805,992 | 4,275,650 | **8.55** |
| BOLL(20,2,2) | 138,524 | 1,575,475 | 7,617,664 | **15.24** |
| ATR(14) | 64,123 | 757,576 | 3,878,053 | **7.76** |

### Streaming vs Native Batch (10K)

| Indicator | Streaming | Native | Faster? |
|-----------|-----------|--------|---------|
| SMA(20) | 64,943 | 94,580 | Streaming 1.5x faster |
| EMA(12) | 29,595 | 22,940 | Native 1.3x faster |
| RSI(14) | 71,990 | 56,120 | Native 1.3x faster |
| MACD | 70,140 | 106,508 | Streaming 1.5x faster |
| BOLL | 138,524 | 440,342 | Streaming 3.2x faster |
| ATR | 64,123 | 50,164 | Native 1.3x faster |

---

## 4. Optimization Targets

### Priority 1: Native SMA — O(n*period) → O(n) with prefix sum
- **Current:** 5,774 µs (500K), 11.55 ns/val
- **Target:** < 350 µs (500K), < 0.7 ns/val (match TA-Lib)
- **Approach:** Running sum / prefix sum algorithm, SIMD vectorization

### Priority 2: Native EMA — cache-friendly scalar loop
- **Current:** 2,018 µs (500K), 4.04 ns/val
- **Target:** < 710 µs (500K), < 1.42 ns/val (match TA-Lib)
- **Approach:** Eliminate Array1 allocation overhead, use slice-based output

### Priority 3: Formula engine overhead reduction
- **Current:** 1.06x - 3.46x overhead vs native
- **Target:** < 1.3x for all indicators
- **Approach:** Stack-pooling for BytecodeVM, constant folding, native fast-path for builtins

### Priority 4: Streaming microoptimization
- **Current:** SMA 7.23 ns/val, RSI 11.36 ns/val
- **Target:** SMA < 5 ns/val, RSI < 7 ns/val
- **Approach:** Ring buffer for SMA, branchless RSI, Welford variance for BOLL

---

## 5. SIMD Baseline (from formula_bench, 100K elements)

| Operation | SimdOps | Scalar | Speedup |
|-----------|---------|--------|---------|
| add | 38,573 ns | 77,337 ns | 2.0x |
| mul | 39,550 ns | 74,316 ns | 1.9x |
| sma(20) | 211,011 ns | 1,014,756 ns | 4.8x |
| ema(12) | 204,815 ns | 248,694 ns | 1.2x |

---

*This baseline was recorded before optimization work (TASK-014 through TASK-017). See `docs/benchmark-results.md` for post-optimization comparison.*
