# Finkit - Financial Technical Analysis Library

[![CI](https://github.com/coeasy/finkit/actions/workflows/ci.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

**A high-performance Technical Analysis library written in Rust with multi-language and multi-platform support.**

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Quick Start Guide](docs/src/quickstart.md) | 5-minute quick start tutorial |
| [Documentation Index](docs/README.md) | Full usage guide and examples |
| [API Reference](docs/api-reference.md) | Detailed API documentation for all indicators |
| [Development Guide](docs/development.md) | Building, testing, and packaging instructions |
| [Latest Optimization Plan](docs/plan/finkit-latest-status-optimization-plan.md) | Current code audit, release blockers and prioritized improvements |
| [Installation Guide](docs/installation.md) | Detailed installation for each language |
| [Python Wheel Guide](docs/python.md) | CPython wheel matrix, source builds, and troubleshooting |
| [Performance Benchmarks](docs/benchmark-results.md) | Performance comparison with TA-Lib |
| [Benchmark vs TA-Lib](docs/BENCHMARK_VS_TALIB.md) | How to read the `bench-vs-talib` output |
| [Indicator List](docs/indicators.md) | Complete list of all indicators |

## Features

- **High Performance**: Core indicators 1.3x–3.2x faster than TA-Lib C (real FFI benchmarks), SIMD-accelerated math
- **150+ Batch Indicators**: Complete coverage of overlap, momentum, volume, volatility, cycle, price transform, and statistics
- **145 Registered Streaming Indicator Entries**: O(1) per-bar incremental updates with checkpoint/restore; the count is generated from docs/indicator_registry.json
- **60+ Candlestick Patterns**: Full recognition of common candlestick patterns
- **15+ Chart Patterns**: Head & Shoulders, Double Top/Bottom, Triangles, Wedges, etc.
- **Formula Engine**: Expression-based computation (`MA(CLOSE, 20)`) with AST/Bytecode optimization, range/last/incremental execution, and bounded zero-copy fast paths
- **Transform Pipeline**: Composable data transforms (LogReturn, ZScore, MinMaxScaler)
- **Builder API**: Type-safe fluent builders for all streaming indicators
- **Multi-Language Support**: Python, Node.js, Java, Go, .NET, C++, Rust, WebAssembly, CLI
- **Cross-Platform**: Linux, macOS, Windows, Android, iOS
- **Multi-Package Install**: build locally or install from source; published artifacts are gated by matching CI and release metadata (see [installation guide](docs/installation.md))

## ⚡ One-Click Build

Build every language binding, install it locally, and run a smoke test —
all from a single command. See [docs/development.md](docs/development.md)
for build instructions.

```bash
./build-usage.sh                  # bash / zsh / Git-Bash
pwsh ./build-usage.ps1            # PowerShell 7+
make                              # make target

# Or one-command Docker (no host toolchain needed)
docker build -t finkit/builder:latest .
docker run --rm -v "$(pwd)/dist:/work/dist" finkit/builder:latest

# One-command Finkit vs TA-Lib head-to-head
./scripts/bench-vs-talib.sh --precision
```

Outputs land in `dist/` (artifacts) and `dist/bench/` (bench report).
See [docs/BENCHMARK_VS_TALIB.md](docs/BENCHMARK_VS_TALIB.md) for how to
read the results.

## Quick Start

> **Python wheels:** CPython 3.8–3.14 wheels are built and tested by GitHub Actions.
> Version tags automatically upload the complete wheel set to the matching GitHub Release.
> For the current v0.1.2 backfill status and installation choices, see the
> [Python Wheel Guide](docs/python.md).

### Python

If the `v0.1.2` GitHub Release is available, download a matching
`finkit-0.1.2-*.whl` from [Releases](https://github.com/coeasy/finkit/releases).
Until then, use a successful [Python wheels workflow](https://github.com/coeasy/finkit/actions/workflows/python-wheels.yml)
artifact or build from source:

```bash
python -m pip install ./finkit-0.1.2-<matching-wheel>.whl
```

See [docs/python.md](docs/python.md) for source builds and wheel selection.

```python
import finkit as ta
import numpy as np

close = np.arange(1.0, 101.0)
sma = ta.sma(close, timeperiod=14)
rsi = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
```

### Node.js

```bash
npm install finkit
```

```typescript
import { sma, rsi, macd } from 'finkit';

const close = Array.from({ length: 100 }, (_, i) => i + 1);
const smaResult = sma(close, 14);
const rsiResult = rsi(close, 14);
const macdResult = macd(close, 12, 26, 9);
```

### Rust

```toml
[dependencies]
finkit = "0.1.2"
```

```rust
use finkit::indicators;
use finkit::math::moving_avg;

let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
let sma = moving_avg::sma(&close, 3).unwrap();
let rsi = indicators::rsi(&close, 14).unwrap();
```

### Go

```bash
go get github.com/coeasy/finkit/go/ta
```

```go
import "github.com/coeasy/finkit/go/ta"

close := []float64{1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0}
sma, _ := ta.SMA(close, 3)
rsi, _ := ta.RSI(close, 14)
```

### Java (Maven)

```xml
<dependency>
    <groupId>com.finkit</groupId>
    <artifactId>finkit</artifactId>
    <version>0.1.2</version>
</dependency>
```

```java
import com.finkit.Indicators;

double[] close = new double[100];
for (int i = 0; i < 100; i++) close[i] = 1.0 + i * 0.09;

double[] sma = Indicators.sma(close, 14);
double[] rsi = Indicators.rsi(close, 14);
```

### .NET

```bash
dotnet add package finkit
```

```csharp
using Finkit;

var close = Enumerable.Range(1, 100).Select(i => (double)i).ToArray();
var sma = Indicators.SMA(close, 14);
var rsi = Indicators.RSI(close, 14);
```

### CLI

```bash
cargo install finkit-cli
```

`finkit-cli` ships with 14 subcommands covering 100+ indicators, OHLCV pattern detection, and a TongDaXin-compatible formula engine.

| Subcommand | Description | Required Input | Example |
|------------|-------------|----------------|---------|
| `sma` | Simple Moving Average | close CSV | `finkit-cli sma -i data.csv --period 14` |
| `ema` | Exponential Moving Average | close CSV | `finkit-cli ema -i data.csv --period 14` |
| `wma` | Weighted Moving Average | close CSV | `finkit-cli wma -i data.csv --period 14` |
| `rsi` | Relative Strength Index | close CSV | `finkit-cli rsi -i data.csv --period 14` |
| `macd` | MACD | close CSV | `finkit-cli macd -i data.csv --fast 12 --slow 26 --signal 9` |
| `bbands` | Bollinger Bands | OHLCV CSV | `finkit-cli bbands -i data.csv --period 20 --stddev 2.0` |
| `atr` | Average True Range | OHLCV CSV | `finkit-cli atr -i data.csv --period 14` |
| `stoch` | Stochastic Oscillator | OHLCV CSV | `finkit-cli stoch -i data.csv` |
| `adx` | Average Directional Index | OHLCV CSV | `finkit-cli adx -i data.csv --period 14` |
| `cci` | Commodity Channel Index | OHLCV CSV | `finkit-cli cci -i data.csv --period 14` |
| `obv` | On Balance Volume | OHLCV CSV | `finkit-cli obv -i data.csv` |
| `willr` | Williams %R | OHLCV CSV | `finkit-cli willr -i data.csv --period 14` |
| `pattern` | Candlestick / chart pattern detection | OHLCV CSV | `finkit-cli pattern -i data.csv --kind candlestick --name doji` |
| `formula` | TongDaXin-compatible formula evaluation | OHLCV CSV | `finkit-cli formula "MA(CLOSE,5) + 2*STDDEV(CLOSE,5)" -i data.csv` |

All subcommands support `--format {plain,csv,json}` and write to stdout or `--output <file>`. Pipe-friendly via stdin.

## Installation

> Registry packages are **not yet published**. Build locally or follow
> [docs/installation.md](docs/installation.md) for source / `dist/` installs.

Detailed installation instructions for each language binding are available in [docs/installation.md](docs/installation.md).

| Language | Package Manager | Command |
|----------|----------------|---------|
| Python   | pip            | Install a v0.1.2 Release wheel or verified Actions artifact |
| Node.js  | npm            | `npm install finkit` |
| Rust     | cargo          | `cargo add finkit` |
| Java     | Maven          | Add dependency to `pom.xml` |
| Go       | go get         | `go get github.com/coeasy/finkit/go/ta` |
| .NET     | NuGet          | `dotnet add package finkit` |
| C++      | CMake          | Link against `libfinkit_ffi.so` |
| WASM     | npm            | `npm install finkit-wasm` |

## Performance

Real FFI benchmark against TA-Lib C 0.6.4 on 10,000 data points (Criterion.rs, `--release`):

### Core Indicators — All Faster Than TA-Lib C

| Indicator | Finkit (µs) | TA-Lib C (µs) | Speedup |
|-----------|----------|---------------|---------|
| SMA(20)   | 12.28    | 19.98         | **1.63x faster** |
| EMA(12)   | 20.60    | 29.19         | **1.42x faster** |
| RSI(14)   | 26.24    | 54.59         | **2.08x faster** |
| MACD(12,26,9) | 30.34 | 98.21         | **3.24x faster** |
| BOLL(20,2)| 46.51    | 55.46         | **1.19x faster** |
| ATR(14)   | 39.00    | 60.60         | **1.55x faster** |

> Methodology and current summary: [docs/benchmark-results.md](docs/benchmark-results.md)

### Extended Indicators — Watch-List (within 25% of TA-Lib)

| Indicator | Finkit (µs) | TA-Lib C (µs) | Verdict |
|-----------|----------|---------------|---------|
| AROON(14) | 63.44    | 52.11         | ⚠️ 0.82x |
| MFI(14)   | 51.23    | 44.17         | ⚠️ 0.86x |
| WILLR(14) | 59.58    | 53.10         | ⚠️ 0.89x |
| WMA(20)   | 22.96    | 20.74         | ⚠️ 0.90x |
| AD        | 13.75    | 13.20         | ⚠️ 0.96x |
| KAMA(30)  | 30.58    | 29.80         | ⚠️ 0.97x |
| OBV       | 10.99    | 10.86         | ⚠️ 0.99x |
| ADOSC(3,10)| 25.44   | 25.36         | ⚠️ 1.00x |
| STOCHF(14,3)| 88.25  | 74.81         | ⚠️ 0.85x |

> Methodology and current summary: [docs/benchmark-results.md](docs/benchmark-results.md)

### Language Binding Overhead

| Language | Relative Performance | Notes |
|----------|---------------------|-------|
| Rust     | 1.00x (baseline)    | Native performance |
| C++ (FFI)| 1.01x               | Minimal overhead |
| Go (CGO) | 1.05x               | CGO call overhead |
| Java (JNI)| 1.08x              | JNI transition cost |
| Python   | 1.10x               | PyO3 zero-copy NumPy |
| Node.js  | 1.12x               | NAPI-RS serialization |
| .NET     | 1.10x               | P/Invoke overhead |
| WASM     | 1.50x               | WebAssembly sandbox |

### Competitive Comparison: Finkit vs TA-Lib vs Kand vs quantedge-ta

| Feature | **Finkit** (Rust) | **TA-Lib** (C) | **Kand** (Rust) | **quantedge-ta** (Rust) |
|---------|--------------|----------------|-----------------|------------------------|
| **Batch Indicators** | 150+ | 150+ | ~30 | ~40 |
| **Streaming** | 98 (O(1)/bar) | ❌ | ~30 | ❌ |
| **FFI Bindings** | Python, Node, Go, Java, .NET, C, WASM | C only | Python | ❌ |
| **Formula Engine** | ✅ JIT + SIMD | ❌ | ❌ | ❌ |
| **Candlestick Patterns** | 60+ | 60+ | ❌ | ❌ |
| **Chart Patterns** | 15+ | ❌ | ❌ | ❌ |
| **Transform Pipeline** | ✅ | ❌ | ❌ | ❌ |
| **Checkpoint/Restore** | ✅ serde | ❌ | ❌ | ❌ |
| **Polars Integration** | ✅ zero-copy | ❌ | ❌ | ❌ |
| **Parallel Sweep** | ✅ rayon | ❌ | ❌ | ❌ |

#### Performance Comparison (10K points, ns/bar)

| Library | SMA(20) | EMA(12) | RSI(14) | MACD | Notes |
|---------|---------|---------|---------|------|-------|
| **Finkit** | **1.26** | **2.05** | **2.67** | **3.14** | Optimized Rust |
| TA-Lib C | 2.04 | 2.97 | 5.52 | 10.00 | Native C via FFI |
| Kand | ~7 | ~7 | ~15 | ~20 | PyO3 optimized |
| quantedge-ta | ~5 | ~4 | ~12 | ~15 | Batch mode only |

## Indicators

### Overlap Studies (15)
SMA, EMA, WMA, DEMA, TEMA, KAMA, MAMA, T3, BBANDS, SAR, HT_TRENDLINE, MIDPOINT, MIDPRICE, MAVP, TRIMA

### Momentum Indicators (22)
RSI, MACD, STOCH, STOCHF, ADX, AROON, AROONOSC, CCI, CMO, MOM, ROC, WILLR, APO, BOP, DX, MFI, MINUS_DI, MINUS_DM, PLUS_DI, PLUS_DM, TRIX, ULTOSC

### Volume Indicators (4)
AD, ADOSC, OBV, CMF

### Volatility Indicators (4)
ATR, NATR, TRANGE, KAMA_VOLATILITY

### Cycle Indicators (6)
HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, HT_TRENDMODE, HT_MEASUREMENT

### Price Transform (5)
AVGPRICE, MEDPRICE, TYPPRICE, WCLPRICE, MEDIPRICE

### Statistics (5)
STDDEV, VAR, LINEARREG, LINEARREG_ANGLE, LINEARREG_INTERCEPT, LINEARREG_SLOPE, TSF, ZSCORE, CORREL

### Pattern Recognition

#### Candlestick Patterns (60+)
CDL2CROWS, CDL3BLACKCROWS, CDL3INSIDE, CDL3OUTSIDE, CDL3STARSINSOUTH, CDL3WHITESOLDIERS, CDLABANDONEDBABY, CDLADVANCEBLOCK, CDLBELTHOLD, CDLBREAKAWAY, CDLCLOSINGMARUBOZU, CDLCONCEALBABYSWALL, CDLCOUNTERATTACK, CDLDARKCLOUDCOVER, CDLDOJI, CDLDOJISTAR, CDLDRAGONFLYDOJI, CDLENGULFING, CDLEVENINGDOJISTAR, CDLEVENINGSTAR, CDLGAPSIDESIDEWHITE, CDLGRAVESTONEDOJI, CDLHAMMER, CDLHANGINGMAN, CDLHARAMI, CDLHARAMICROSS, CDLHIGHWAVE, CDLHIKKAKE, CDLHIKKAKEMOD, CDLHOMINGPIGEON, CDLIDENTICAL3CROWS, CDLINNECK, CDLINVERTEDHAMMER, CDLKICKING, CDLKICKINGBYLENGTH, CDLLADDERBOTTOM, CDLLONGLEGGEDDOJI, CDLLONGLINE, CDLMARUBOZU, CDLMATCHINGLOW, CDLMATHOLD, CDLMORNINGDOJISTAR, CDLMORNINGSTAR, CDLONNECK, CDLPIERCING, CDLRICKSHAWMAN, CDLRISEFALL3METHODS, CDLSEPARATINGLINES, CDLSHOOTINGSTAR, CDLSHORTLINE, CDLSPINNINGTOP, CDLSTALLEDPATTERN, CDLSTICKSANDWICH, CDLTAKURI, CDLTASUKIGAP, CDLTHRUSTING, CDLTRISTAR, CDLUNIQUE3RIVER, CDLUPSIDEGAP2CROWS, CDLXSIDEGAP3METHODS

#### Chart Patterns (15+)
Head & Shoulders Top/Bottom, Double Top/Bottom, Triple Top/Bottom, Ascending/Descending Triangle, Symmetrical Triangle, Rising/Falling Wedge, Pennant, Flag, Rectangle, Rounding Top/Bottom

Full indicator list with parameters: [docs/indicators.md](docs/indicators.md)

## Usage Examples

### Python - Complete Example

```python
import finkit as ta
import numpy as np
import pandas as pd

# Generate sample OHLCV data
dates = pd.date_range('2024-01-01', periods=100, freq='D')
close = np.cumsum(np.random.randn(100)) + 100
high = close + np.random.uniform(0.5, 2.0, 100)
low = close - np.random.uniform(0.5, 2.0, 100)
open_ = close + np.random.uniform(-1.0, 1.0, 100)
volume = np.random.uniform(1000, 5000, 100)

# Overlap Studies
sma_20 = ta.sma(close, timeperiod=20)
sma_50 = ta.sma(close, timeperiod=50)
ema_12 = ta.ema(close, timeperiod=12)
bbands = ta.bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0)

# Momentum Indicators
rsi = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
stoch_k, stoch_d = ta.stoch(high, low, close, fastk_period=5, slowk_period=3, slowd_period=3)

# Volatility
atr = ta.atr(high, low, close, timeperiod=14)

# Volume
obv = ta.obv(close, volume)

# Pattern Recognition
doji = ta.cdl_doji(open_, high, low, close, doji_pct=0.1)
hammer = ta.cdl_hammer(open_, high, low, close)
engulfing = ta.cdl_engulfing(open_, high, low, close)

# Chart Patterns
double_tops = ta.detect_double_top(high, lookback=20, tolerance=0.03)
head_shoulders = ta.detect_head_shoulders(high, lookback=30, tolerance=0.05)
```

### Node.js - Complete Example

```typescript
import {
  sma, ema, rsi, macd, bollinger_bands,
  stoch, atr, obv,
  cdl_doji, cdl_hammer, cdl_engulfing,
  detect_double_top, detect_head_shoulders
} from 'finkit';

// Generate sample data
const close = Array.from({ length: 100 }, (_, i) => 100 + i * 0.5 + Math.random() * 2 - 1);
const high = close.map(x => x + Math.random() * 2 + 0.5);
const low = close.map(x => x - Math.random() * 2 - 0.5);
const open = close.map(x => x + Math.random() * 2 - 1);
const volume = Array.from({ length: 100 }, () => Math.random() * 4000 + 1000);

// Calculate indicators
const sma20 = sma(close, 20);
const ema12 = ema(close, 12);
const rsi14 = rsi(close, 14);
const macdResult = macd(close, 12, 26, 9);
const bbandsResult = bollinger_bands(close, 20, 2.0, 2.0);
const stochResult = stoch(high, low, close, 5, 3, 3);
const atr14 = atr(high, low, close, 14);
const obvResult = obv(close, volume);

// Pattern recognition
const doji = cdl_doji(open, high, low, close, 0.1);
const hammer = cdl_hammer(open, high, low, close);
const doubleTops = detect_double_top(high, 20, 0.03);
const headShoulders = detect_head_shoulders(high, 30, 0.05);
```

### Java - Complete Example

```java
import com.finkit.Indicators;
import com.finkit.MacdResult;
import com.finkit.BbandsResult;
import com.finkit.StochResult;
import java.util.Arrays;

public class Example {
    public static void main(String[] args) {
        // Generate sample data
        double[] close = new double[100];
        double[] high = new double[100];
        double[] low = new double[100];
        double[] open = new double[100];
        double[] volume = new double[100];

        for (int i = 0; i < 100; i++) {
            close[i] = 100 + i * 0.5 + Math.random() * 2 - 1;
            high[i] = close[i] + Math.random() * 2 + 0.5;
            low[i] = close[i] - Math.random() * 2 - 0.5;
            open[i] = close[i] + Math.random() * 2 - 1;
            volume[i] = Math.random() * 4000 + 1000;
        }

        // Calculate indicators
        double[] sma20 = Indicators.sma(close, 20);
        double[] ema12 = Indicators.ema(close, 12);
        double[] rsi14 = Indicators.rsi(close, 14);

        MacdResult macd = Indicators.macd(close, 12, 26, 9);
        System.out.println("MACD: " + Arrays.toString(macd.macd));

        BbandsResult bbands = Indicators.bbands(close, 20, 2.0, 2.0);
        StochResult stoch = Indicators.stoch(high, low, close, 5, 3, 3);

        double[] atr14 = Indicators.atr(high, low, close, 14);
        double[] obv = Indicators.obv(close, volume);

        // Pattern recognition
        int[] doji = Indicators.cdlDoji(open, high, low, close, 0.1);
        int[] hammer = Indicators.cdlHammer(open, high, low, close);
        int[] doubleTops = Indicators.detectDoubleTop(high, 20, 0.03);
    }
}
```

### Go - Complete Example

```go
package main

import (
    "fmt"
    "math/rand"
    "github.com/coeasy/finkit/go/ta"
)

func main() {
    // Generate sample data
    close := make([]float64, 100)
    high := make([]float64, 100)
    low := make([]float64, 100)
    volume := make([]float64, 100)

    for i := 0; i < 100; i++ {
        close[i] = 100 + float64(i)*0.5 + rand.Float64()*2 - 1
        high[i] = close[i] + rand.Float64()*2 + 0.5
        low[i] = close[i] - rand.Float64()*2 - 0.5
        volume[i] = rand.Float64()*4000 + 1000
    }

    // Calculate indicators
    sma20, _ := ta.SMA(close, 20)
    ema12, _ := ta.EMA(close, 12)
    rsi14, _ := ta.RSI(close, 14)

    macd, signal, hist, _ := ta.MACD(close, 12, 26, 9)
    fmt.Printf("MACD length: %d\n", len(macd))

    upper, middle, lower, _ := ta.BBands(close, 20, 2.0, 2.0)
    fastK, slowD, _ := ta.Stoch(high, low, close, 5, 3, 3)

    atr14, _ := ta.ATR(high, low, close, 14)
    obv, _ := ta.OBV(close, volume)

    // Pattern recognition
    doji, _ := ta.CDLDoji(close, high, low, close, 0.1)
    hammer, _ := ta.CDLHammer(close, high, low, close)
}
```

### .NET - Complete Example

```csharp
using System;
using System.Linq;
using Finkit;

class Program
{
    static void Main()
    {
        // Generate sample data
        var close = Enumerable.Range(0, 100)
            .Select(i => 100 + i * 0.5 + new Random().NextDouble() * 2 - 1)
            .ToArray();
        var high = close.Select(x => x + new Random().NextDouble() * 2 + 0.5).ToArray();
        var low = close.Select(x => x - new Random().NextDouble() * 2 - 0.5).ToArray();
        var open = close.Select(x => x + new Random().NextDouble() * 2 - 1).ToArray();
        var volume = Enumerable.Range(0, 100)
            .Select(_ => new Random().NextDouble() * 4000 + 1000)
            .ToArray();

        // Calculate indicators
        var sma20 = Indicators.SMA(close, 20);
        var ema12 = Indicators.EMA(close, 12);
        var rsi14 = Indicators.RSI(close, 14);

        var macd = Indicators.MACD(close, 12, 26, 9);
        Console.WriteLine($"MACD length: {macd.Macd.Length}");

        var bbands = Indicators.BBands(close, 20, 2.0, 2.0);
        var stoch = Indicators.Stoch(high, low, close, 5, 3, 3);

        var atr14 = Indicators.ATR(high, low, close, 14);
        var obv = Indicators.OBV(close, volume);

        // Pattern recognition
        var doji = Indicators.CDLDoji(open, high, low, close, 0.1);
        var hammer = Indicators.CDLHammer(open, high, low, close);
        var doubleTops = Indicators.DetectDoubleTop(high, 20, 0.03);
    }
}
```

### WebAssembly (WASM)

```bash
npm install finkit-wasm
```

```javascript
import init, { sma, ema, rsi, macd } from 'finkit-wasm';

await init();
const close = new Float64Array([44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84]);
const smaResult = sma(close, 3);
const rsiResult = rsi(close, 5);
```

### C/C++

```bash
# Build the shared library
cargo build --release -p finkit-ffi
```

```c
#include "finkit.h"

double close[] = {44.34, 44.09, 43.61, 44.33, 44.83, 45.10};
TaResult *result = ta_sma(close, 6, 3);
// use result->data, result->length
ta_free_result(result);
```

## One-Click Build Scripts

### Build & Test (Development)

```powershell
# Windows
.\build.ps1 all          # Build & test all targets

# Linux / macOS
./build.sh all            # Build & test all targets
```

### Generate Distributable Packages

Build all language installation packages into `dist/`:

```powershell
# Windows (PowerShell)
.\build-packages.ps1                    # Build all languages
.\build-packages.ps1 -Lang python       # Python .whl only
.\build-packages.ps1 -Lang java,c       # Java + C packages

# Linux / macOS (Bash)
./build-packages.sh                     # Build all languages
./build-packages.sh python              # Python .whl only
./build-packages.sh java c              # Java + C packages
```

Output structure:
```
dist/
├── python/    # .whl (pip install)
├── java/      # .jar + native .dll/.so
├── node/      # npm package
├── go/        # .so/.dll shared library
├── c/         # static/shared lib + headers
└── dotnet/    # .nupkg
```

### Prerequisites

| Target | Requirements |
|--------|-------------|
| core   | Rust toolchain |
| python | Rust + [maturin](https://github.com/PyO3/maturin) (`pip install maturin`) |
| node   | Rust + Node.js + [napi-rs](https://napi.rs/) (`npm install -g @napi-rs/cli`) |
| go     | Rust + Go toolchain |
| java   | Rust + Maven (optional for JAR) |
| c      | Rust toolchain |
| dotnet | Rust + .NET SDK |

## Architecture

```
finkit/
├── core/                       # Core Rust library
│   └── src/
│       ├── indicators/         # Technical indicators
│       │   ├── overlap.rs      # SMA, EMA, BBANDS, SAR, etc.
│       │   ├── momentum.rs     # RSI, MACD, STOCH, ADX, etc.
│       │   ├── volume.rs       # OBV, AD, ADOSC, CMF
│       │   ├── volatility.rs   # ATR, NATR, TRANGE
│       │   ├── cycle.rs        # HT_DCPERIOD, HT_SINE, etc.
│       │   ├── price_transform.rs  # AVGPRICE, TYPPRICE, etc.
│       │   └── statistics.rs   # STDDEV, VAR, LINEARREG, ZSCORE
│       ├── math/               # Mathematical foundation
│       │   ├── moving_avg.rs   # SMA, EMA, WMA, DEMA, TEMA, KAMA
│       │   ├── linear.rs       # Linear regression
│       │   └── statistics.rs   # Mean, variance, correlation
│       └── patterns/           # Pattern recognition
│           ├── candlestick.rs  # 60+ candlestick patterns
│           └── chart.rs        # 15+ chart patterns
├── ffi/                        # FFI bindings
│   ├── c-binding/              # C FFI (base layer)
│   ├── python-binding/         # Python (PyO3)
│   ├── node-binding/           # Node.js (NAPI-RS)
│   ├── java-binding/           # Java (JNI)
│   ├── go-binding/             # Go (CGO)
│   └── dotnet-binding/         # .NET (P/Invoke)
├── cli/                        # Command-line interface
├── wasm/                       # WebAssembly module
└── visualization/              # Visualization module
```

## Streaming Indicators

All 98 streaming indicators support O(1) per-bar incremental updates:

```rust
use finkit::streaming::{StreamingSma, StreamingRsi, StreamingIndicator};

let mut sma = StreamingSma::new(20);
let mut rsi = StreamingRsi::new(14);

for bar in market_data {
    let sma_val = sma.next(bar.close);     // Option<f64>
    let rsi_val = rsi.next(bar.close);     // Option<f64>
}
```

Builder pattern:
```rust
let sma = StreamingSma::builder().period(20).build();
```

Checkpoint/restore (with `serde` feature):
```rust
let state = sma.save_state()?;
let restored = StreamingSma::restore_state(&state)?;
```

## Transform Pipeline

```rust
use finkit::transforms::{Pipeline, LogReturn, ZScore, Transform};

let result = Pipeline::new()
    .add(LogReturn)
    .add(ZScore)
    .transform(&data);
```

## Formula Engine

```rust
use finkit::formula::{FormulaEngine, FormulaContext};

let mut engine = FormulaEngine::new();
let result = engine.eval("MA(CLOSE, 20) + 2 * STD(CLOSE, 20)", &mut ctx)?;
```

## Documentation

- [Complete Indicator List](docs/indicators.md) - All indicators with parameters and descriptions
- [Installation Guide](docs/installation.md) - Detailed installation for each language
- [API Reference](docs/api-reference.md) - Full API documentation
- [Benchmark Results](docs/benchmark-results.md) - Detailed performance analysis vs TA-Lib C
- [Development Guide](docs/development.md) - How to contribute and build from source
- [Contributing Guidelines](CONTRIBUTING.md) - How to contribute to the project

## Development

### Build from Source

```bash
# Clone the repository
git clone https://github.com/coeasy/finkit
cd finkit

# Build core library
cargo build --release

# Run tests
cargo test

# Build Python bindings
cd ffi/python-binding
maturin develop

# Build Node.js bindings
cd ffi/node-binding
npm install
npm run build

# Build Java bindings
cd ffi/java-binding
cargo build --release

# Build Go binding
cd ffi/go-binding
make build

# Build .NET binding
cd ffi/dotnet-binding
cargo build --release

# Build CLI
cd cli
cargo install --path .

# Build WebAssembly
cd wasm
wasm-pack build --target web
```

### CI/CD

CI runs on every push and pull request to `main`:

- `cargo fmt --check`
- `cargo clippy` (core hard gate; workspace advisory)
- `cargo test -p finkit` + doc tests
- `cargo doc` / `cargo audit`

Current workflows:
- **ci.yml** — Rust format, clippy, version consistency, core tests, docs and dependency audit
- **python-wheels.yml** — ABI3 wheel build, CPython compatibility matrix, wheel metadata validation and tag-triggered upload
- **docs-check.yml** — Generated SSOT and release-version consistency
- Python wheel build matrix (CPython 3.8–3.14 on four platform targets); see [python-wheels.yml](.github/workflows/python-wheels.yml)

Planned follow-ups:
- **perf-gate.yml** — Performance regression detection against a checked-in baseline
- **fuzz.yml** — Scheduled and manual fuzzing

## 可视化模块

Finkit 内置高性能 K 线图可视化模块，支持纯 Rust 渲染，无需浏览器或外部依赖。

### 功能概述

- **4 种图表类型**：Candlestick（蜡烛图）、OHLC Bar（美国线）、Line（折线图）、Area（面积图）
- **8 种技术指标叠加**：MA、EMA、BOLL、MACD、RSI、KDJ、SAR、自定义指标
- **多语言支持**：中文（zh-CN）/ 英文（en-US），自动切换涨跌配色
- **双主题**：Light / Dark 主题一键切换
- **3 种输出格式**：SVG、PNG、HTML（含交互式十字光标）
- **大数据优化**：LTTB / MinMax / EveryNth 降采样，百万级数据流畅渲染
- **增量渲染**：实时追加 K 线数据，仅重绘变化部分

### 快速开始（Rust）

```toml
[dependencies]
finkit-visualization = { version = "0.1.2", features = ["svg"] }
```

```rust
use finkit_visualization::chart::KlineChart;
use finkit_visualization::config::{ChartConfigBuilder, ChartType, IndicatorConfig, IndicatorType};
use finkit_visualization::data::KlineData;
use finkit_visualization::language::Language;

fn main() {
    let data = KlineData::new(
        vec!["2024-01-01".into(), "2024-01-02".into(), "2024-01-03".into()],
        vec![100.0, 102.0, 101.0],
        vec![105.0, 106.0, 104.0],
        vec![98.0, 100.0, 99.0],
        vec![103.0, 104.0, 100.0],
        vec![1000.0, 1200.0, 800.0],
    );

    let config = ChartConfigBuilder::new()
        .with_title("上证指数")
        .with_language(Language::ZhCn)
        .with_chart_type(ChartType::Candlestick)
        .with_dimensions(1200, 600)
        .build();

    let indicators = vec![
        IndicatorConfig::new(IndicatorType::MA, vec![5.0, 10.0, 20.0]),
        IndicatorConfig::new(IndicatorType::MACD, vec![12.0, 26.0, 9.0]),
        IndicatorConfig::new(IndicatorType::RSI, vec![14.0]),
    ];

    let mut chart = KlineChart::new(config);
    chart.build_draw_list(&data, &indicators).unwrap();
    chart.save_as_svg("kline.svg").unwrap();
}
```

### 多语言示例

#### Python

```python
import finkit as ta

data = ta.PyKlineData(
    dates=["2024-01-01", "2024-01-02", "2024-01-03"],
    opens=[100.0, 102.0, 101.0],
    highs=[105.0, 106.0, 104.0],
    lows=[98.0, 100.0, 99.0],
    closes=[103.0, 104.0, 100.0],
    volumes=[1000.0, 1200.0, 800.0],
)

chart = ta.PyKlineChart(data, language="zh", title="上证指数")
chart.add_ma([5, 10, 20])
chart.add_macd(12, 26, 9)
chart.add_rsi(14)
chart.save_as_svg("kline.svg")
chart.save_as_html("kline.html")
```

#### Node.js

```typescript
import { PyKlineChart } from 'finkit';

// 使用 JSON 数据创建图表
const chartData = {
  dates: ['2024-01-01', '2024-01-02', '2024-01-03'],
  opens: [100.0, 102.0, 101.0],
  highs: [105.0, 106.0, 104.0],
  lows: [98.0, 100.0, 99.0],
  closes: [103.0, 104.0, 100.0],
  volumes: [1000.0, 1200.0, 800.0],
};
```

#### Java

```java
import com.finkit.KlineChart;
import com.finkit.KlineData;

KlineData data = new KlineData(
    new String[]{"2024-01-01", "2024-01-02", "2024-01-03"},
    new double[]{100.0, 102.0, 101.0},
    new double[]{105.0, 106.0, 104.0},
    new double[]{98.0, 100.0, 99.0},
    new double[]{103.0, 104.0, 100.0},
    new double[]{1000.0, 1200.0, 800.0}
);

KlineChart chart = new KlineChart(data, "zh", "上证指数", 1200, 600);
chart.addMA(new int[]{5, 10, 20});
chart.addMACD(12, 26, 9);
chart.addRSI(14);
chart.saveAsSvg("kline.svg");
```

### 输出格式

| 格式 | Feature | 说明 |
|------|---------|------|
| SVG  | `svg`   | 矢量图，无损缩放，适合嵌入文档和网页 |
| PNG  | `png`   | 位图，适合截图和报告 |
| HTML | `html`  | 自包含 HTML 文件，内置交互式十字光标、缩放和平移 |

### 性能指标

| 数据量 | SVG 渲染时间 | 说明 |
|--------|-------------|------|
| 1,000 根 K 线 | < 5ms | 实时渲染 |
| 10,000 根 K 线 | < 50ms | 流畅渲染 |
| 100,000 根 K 线（降采样后） | < 100ms | 自动降采样至画布宽度 |
| 1,000,000 根 K 线降采样 | < 100ms | LTTB 算法 |

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Acknowledgments

- Original TA-Lib by Mario Fortier
- All contributors to the technical analysis community
