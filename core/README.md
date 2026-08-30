# alpha-ta-core

> **Tier1** · Rust native · 成熟度: `stable` · [绑定分级说明](../docs/BINDING_TIERS.md)

| 能力 | 指标计算 | 流式 | 公式引擎 | ML 特征 | 可视化 |
|------|----------|------|----------|---------|--------|
| 状态 | ✅ 完整 | ✅ | ✅ | ✅ | ✅ |

High-performance technical analysis library for Rust — faster than TA-Lib C.

## Features

- **177 Indicators**: SMA, EMA, RSI, MACD, Bollinger Bands, ATR, and many more
- **60+ Candlestick Patterns**: Doji, Hammer, Engulfing, Morning Star, etc.
- **15+ Chart Patterns**: Head & Shoulders, Double Top/Bottom, Triangles
- **98 Streaming Indicators**: O(1) per-bar incremental computation
- **Formula Engine**: Expression-based indicator computation (optional `formula` feature)
- **Transform Pipeline**: Composable data transformers (`LogReturn`, `ZScore`, etc.)
- **Builder API**: Fluent indicator configuration with price source selection
- **SIMD Acceleration**: AVX2/NEON optimized math operations
- **Checkpoint/Restore**: Serialize and restore indicator state
- **FFI Bindings**: Python, Node.js, Go, Java, C, .NET, WASM

## Performance — AlphaTA vs Competitors (10K data points)

| Indicator | AlphaTA (ns/bar) | TA-Lib C | Kand | quantedge-ta | AlphaTA vs TA-Lib |
|-----------|-------------|----------|------|-------------|---------------|
| SMA(20) | **1.28** | 2.02 | ~2.8 | ~3.5 | **1.6x faster** |
| EMA(12) | **2.07** | 2.97 | ~3.0 | ~4.2 | **1.4x faster** |
| RSI(14) | **2.66** | 5.51 | ~4.5 | ~6.5 | **2.1x faster** |
| MACD(12,26,9) | **9.75** | 10.11 | ~12.0 | ~18.0 | **1.0x faster** |
| BOLL(20,2) | **4.17** | 5.65 | ~8.0 | ~12.0 | **1.4x faster** |
| ATR(14) | **3.98** | 6.13 | ~5.5 | ~8.0 | **1.5x faster** |

> AlphaTA and TA-Lib C data from real FFI benchmarks (`cargo bench --bench talib_comparison_bench`).
> Kand/quantedge-ta from published benchmarks (estimated, denoted with ~).

## Quick Start

```rust
use alpha_ta_core::indicators;

let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

// Simple Moving Average
let sma = indicators::sma(&close, 3).unwrap();

// Relative Strength Index
let rsi = indicators::rsi(&close, 5).unwrap();
```

## Streaming Indicators

```rust
use alpha_ta_core::streaming::{StreamingIndicator, indicators::StreamingSma};

let mut sma = StreamingSma::new(3);
sma.next(1.0); // warming up
sma.next(2.0); // warming up
let val = sma.next(3.0); // Some(2.0)
assert_eq!(val, Some(2.0));
```

## Builder API

```rust
use alpha_ta_core::streaming::{IndicatorBuilder, Builder, StreamingIndicator};
use alpha_ta_core::streaming::indicators::StreamingSma;

let mut sma = StreamingSma::builder()
    .period(14)
    .build()
    .unwrap();

assert!(!sma.is_ready());
for i in 1..=14 {
    sma.next(i as f64);
}
assert!(sma.is_ready());
```

## Transform Pipeline

```rust
use alpha_ta_core::transforms::{Pipeline, LogReturn, ZScore, Transform};

let data = vec![100.0, 105.0, 103.0, 108.0, 110.0, 107.0, 112.0, 115.0, 113.0, 118.0];
let result = Pipeline::new()
    .add(LogReturn)
    .add(ZScore)
    .transform(&data);
assert!(!result.is_empty());
```

## Feature Flags

| Feature | Description |
|---------|-------------|
| `formula` | Enable formula expression engine |
| `talib-c` | Enable C TA-Lib FFI bindings |
| `serde` | Enable checkpoint/restore serialization |
| `AlphaTA-polars` | Enable Polars/Arrow integration |
| `rayon` | Enable parallel sweep engine |
