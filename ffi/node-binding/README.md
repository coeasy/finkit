# @alphata/node

> **Tier1** · 成熟度: `stable` · [绑定分级说明](../../docs/BINDING_TIERS.md)

| 能力 | 指标计算 | 流式 | 公式引擎 | ML 特征 | 可视化 |
|------|----------|------|----------|---------|--------|
| 状态 | ✅ 完整 | ✅ | ✅ | ⚠️ 部分 | ⚠️ 部分 |

High-performance technical analysis library for Node.js, powered by Rust and [NAPI-RS](https://napi.rs).

- **150+ indicators**: Overlap studies, momentum, volume, volatility, cycle, statistical indicators
- **60+ candlestick patterns**: Doji, Hammer, Engulfing, Morning/Evening Star, and more
- **Chart pattern recognition**: Head & Shoulders, Double Top/Bottom, Triple Top/Bottom
- **Native performance**: Rust core with zero-copy FFI, 10-20x faster than pure JS implementations
- **TypeScript support**: Full type definitions included
- **ESM & CommonJS**: Supports both module systems
- **Async computation**: Non-blocking async APIs for large datasets
- **Cross-platform**: macOS, Linux, Windows (x64 & ARM64)

## Installation

```bash
npm install @alphata/node
```

## Quick Start

### CommonJS

```javascript
const ta = require('@alphata/node')

const close = [100.0, 101.5, 99.8, 102.3, 103.1, 101.0, 100.5, 102.8, 104.2, 103.5]

// RSI
const rsi = ta.rsi(close, 14)
console.log('RSI:', rsi[rsi.length - 1])

// MACD
const macd = ta.macd(close, 12, 26, 9)
console.log('MACD:', macd.macd[macd.macd.length - 1])

// Bollinger Bands
const bb = ta.bollinger_bands(close, 5, 2.0, 2.0)
console.log('Upper:', bb.upper[bb.upper.length - 1])
```

### ESM

```javascript
import { rsi, macd, bollingerBands } from '@alphata/node'

const close = [100.0, 101.5, 99.8, 102.3, 103.1]

const rsiValues = rsi(close, 14)
const macdResult = macd(close, 12, 26, 9)
const bbResult = bollingerBands(close, 5, 2.0, 2.0)
```

### TypeScript

```typescript
import type { MacdResult, BbandsResult } from '@alphata/node'
import { rsi, macd, bollinger_bands } from '@alphata/node'

const close: number[] = [100.0, 101.5, 99.8, 102.3, 103.1]

const rsiValues: number[] = rsi(close, 14)
const macdResult: MacdResult = macd(close, 12, 26, 9)
const bbResult: BbandsResult = bollinger_bands(close, 5, 2.0, 2.0)
```

### Async (for large datasets)

```javascript
import { macdAsync } from '@alphata/node'

// Non-blocking computation on large dataset
const largeDataset = generateLargePriceArray(1000000)
const result = await macdAsync(largeDataset, 12, 26, 9)
console.log('MACD computed without blocking event loop')
```

## API Reference

### Overlap Studies

| Function | Description |
|----------|-------------|
| `sma(close, period)` | Simple Moving Average |
| `ema(close, period)` | Exponential Moving Average |
| `wma(close, period)` | Weighted Moving Average |
| `dema(close, period)` | Double Exponential Moving Average |
| `tema(close, period)` | Triple Exponential Moving Average |
| `kama(close, period)` | Kaufman Adaptive Moving Average |
| `mama(close, fastlimit?, slowlimit?)` | MESA Adaptive Moving Average |
| `t3(close, period, vfactor?)` | Triple Exponential Moving Average (T3) |

### Momentum Indicators

| Function | Description |
|----------|-------------|
| `rsi(close, period)` | Relative Strength Index (0-100) |
| `macd(close, fast, slow, signal)` | MACD (returns `{macd, signal, hist}`) |
| `macd_async(close, fast, slow, signal)` | Async MACD for large datasets |
| `stoch(high, low, close, fastk, slowk, slowd)` | Stochastic Oscillator |
| `adx(high, low, close, period)` | Average Directional Index |
| `atr(high, low, close, period)` | Average True Range |
| `cci(high, low, close, period)` | Commodity Channel Index |
| `mom(close, period)` | Momentum |
| `roc(close, period)` | Rate of Change |
| `willr(high, low, close, period)` | Williams %R |
| `apo(close, fast, slow)` | Absolute Price Oscillator |
| `bop(open, high, low, close)` | Balance of Power |
| `cmo(close, period)` | Chande Momentum Oscillator |
| `dx(high, low, close, period)` | Directional Movement Index |
| `mfi(high, low, close, volume, period)` | Money Flow Index |
| `minus_di(high, low, close, period)` | Minus Directional Indicator |
| `plus_di(high, low, close, period)` | Plus Directional Indicator |
| `trix(close, period)` | Triple Exponential Average |
| `aroon(high, low, period)` | Aroon Indicator |

### Volume Indicators

| Function | Description |
|----------|-------------|
| `obv(close, volume)` | On Balance Volume |
| `ad(high, low, close, volume)` | Accumulation/Distribution Line |
| `adosc(high, low, close, volume, fast, slow)` | A/D Oscillator |

### Volatility Indicators

| Function | Description |
|----------|-------------|
| `bollinger_bands(close, period, nbdevup, nbdevdn)` | Bollinger Bands |
| `natr(high, low, close, period)` | Normalized ATR |
| `trange(high, low, close)` | True Range |

### Cycle Indicators (Hilbert Transform)

| Function | Description |
|----------|-------------|
| `ht_dcperiod(close)` | Dominant Cycle Period |
| `ht_dcphase(close)` | Dominant Cycle Phase |
| `ht_phasor(close)` | Phasor Components (in_phase, quadrature) |
| `ht_sine(close)` | Sine Wave (sine, lead_sine) |
| `ht_trendmode(close)` | Trend vs Cycle Mode (1.0/0.0) |
| `ht_trendline(close)` | Instantaneous Trendline |

### Price Transforms

| Function | Description |
|----------|-------------|
| `avgprice(open, high, low, close)` | (O+H+L+C)/4 |
| `medprice(high, low)` | (H+L)/2 |
| `typprice(high, low, close)` | (H+L+C)/3 |
| `wclprice(high, low, close)` | (H+L+2*C)/4 |

### Statistical Indicators

| Function | Description |
|----------|-------------|
| `zscore(input, period)` | Z-Score (standardized values) |
| `percent_rank(input, period)` | Percentile Rank (0-100) |
| `beta(asset, benchmark, period)` | Beta Coefficient |
| `correlation(inputA, inputB, period)` | Pearson Correlation (-1 to 1) |
| `std_dev(input, period, nb_dev)` | Rolling Standard Deviation |
| `linear_reg(input, period)` | Linear Regression values |
| `tsf(input, period)` | Time Series Forecast |

### Candlestick Patterns

All candlestick functions return `number[]` where:
- `100` = Bullish pattern detected
- `-100` = Bearish pattern detected
- `0` = No pattern

| Function | Description |
|----------|-------------|
| `cdl_doji(open, high, low, close, doji_pct)` | Doji |
| `cdl_dragonfly_doji(open, high, low, close, doji_pct)` | Dragonfly Doji |
| `cdl_gravestone_doji(open, high, low, close, doji_pct)` | Gravestone Doji |
| `cdl_long_legged_doji(open, high, low, close, doji_pct)` | Long-Legged Doji |
| `cdl_hammer(open, high, low, close)` | Hammer |
| `cdl_inverted_hammer(open, high, low, close)` | Inverted Hammer |
| `cdl_hanging_man(open, high, low, close)` | Hanging Man |
| `cdl_shooting_star(open, high, low, close)` | Shooting Star |
| `cdl_engulfing(open, high, low, close)` | Engulfing Pattern |
| `cdl_harami(open, high, low, close)` | Harami Pattern |
| `cdl_harami_cross(open, high, low, close)` | Harami Cross |
| `cdl_morning_star(open, high, low, close)` | Morning Star |
| `cdl_evening_star(open, high, low, close)` | Evening Star |
| `cdl_morning_doji_star(open, high, low, close, doji_pct)` | Morning Doji Star |
| `cdl_evening_doji_star(open, high, low, close, doji_pct)` | Evening Doji Star |
| `cdl_three_white_soldiers(open, high, low, close)` | Three White Soldiers |
| `cdl_three_black_crows(open, high, low, close)` | Three Black Crows |
| `cdl_marubozu(open, high, low, close, shadow_pct)` | Marubozu |
| `cdl_piercing(open, high, low, close)` | Piercing Pattern |
| `cdl_dark_cloud_cover(open, high, low, close)` | Dark Cloud Cover |
| `cdl_belt_hold(open, high, low, close)` | Belt Hold |
| `cdl_spinning_top(open, high, low, close)` | Spinning Top |
| `cdl_high_wave(open, high, low, close)` | High Wave |
| `cdl_rickshaw_man(open, high, low, close)` | Rickshaw Man |
| `cdl_tweezer_top(open, high, low, close)` | Tweezer Top |
| `cdl_tweezer_bot(open, high, low, close)` | Tweezer Bottom |
| `cdl_kicking(open, high, low, close)` | Kicking |

### Chart Patterns

| Function | Description |
|----------|-------------|
| `detect_head_shoulders(high, min_bars, head_ratio)` | Head & Shoulders Top |
| `detect_head_shoulders_bottom(low, min_bars, head_ratio)` | Head & Shoulders Bottom |
| `detect_double_top(high, lookback, tolerance)` | Double Top |
| `detect_double_bottom(low, lookback, tolerance)` | Double Bottom |
| `detect_triple_top(high, lookback, tolerance)` | Triple Top |
| `detect_triple_bottom(low, lookback, tolerance)` | Triple Bottom |

## Building from Source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 16
- [Rust](https://rustup.rs/) >= 1.70
- `@napi-rs/cli` >= 2.18

```bash
git clone https://github.com.alphata/AlphaTA.git
cd AlphaTA/ffi/node-binding

# Install dependencies
npm install

# Build (release mode)
npm run build

# Build (debug mode)
npm run build:debug
```

## Platform Support

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | x86_64 | ✅ |
| macOS | Apple Silicon (arm64) | ✅ |
| Linux (glibc) | x86_64 | ✅ |
| Linux (glibc) | arm64 | ✅ |
| Linux (musl) | x86_64 | ✅ |
| Linux (musl) | arm64 | ✅ |
| Windows | x86_64 | ✅ |
| Windows | arm64 | ✅ |

## License

Apache-2.0
