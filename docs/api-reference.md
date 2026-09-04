# API Reference

Complete API reference for all language bindings in Finkit.

## Core API (Rust)

### Module Structure

```rust
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::{candlestick, chart};
```

### Overlap Studies

```rust
/// Simple Moving Average
pub fn sma(data: &[f64], period: usize) -> Result<Vec<f64>>;

/// Exponential Moving Average
pub fn ema(data: &[f64], period: usize) -> Result<Vec<f64>>;

/// Bollinger Bands
pub fn bollinger_bands(
    data: &[f64],
    timeperiod: usize,
    nbdevup: f64,
    nbdevdn: f64
) -> Result<BbandsResult>;

pub struct BbandsResult {
    pub upper: Vec<f64>,
    pub middle: Vec<f64>,
    pub lower: Vec<f64>,
}

/// Parabolic SAR
pub fn sar(
    high: &[f64],
    low: &[f64],
    acceleration: f64,
    maximum: f64
) -> Result<Vec<f64>>;
```

### Momentum Indicators

```rust
/// Relative Strength Index
pub fn rsi(data: &[f64], period: usize) -> Result<Vec<f64>>;

/// MACD
pub fn macd(
    data: &[f64],
    fastperiod: usize,
    slowperiod: usize,
    signalperiod: usize
) -> Result<MacdResult>;

pub struct MacdResult {
    pub macd: Vec<f64>,
    pub signal: Vec<f64>,
    pub hist: Vec<f64>,
}

/// Stochastic
pub fn stoch(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: usize,
    slowk_period: usize,
    slowd_period: usize
) -> Result<StochResult>;

pub struct StochResult {
    pub k: Vec<f64>,
    pub d: Vec<f64>,
}
```

### Volume Indicators

```rust
/// On Balance Volume
pub fn obv(close: &[f64], volume: &[f64]) -> Result<Vec<f64>>;

/// Chaikin A/D Line
pub fn ad(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64]
) -> Result<Vec<f64>>;
```

### Volatility Indicators

```rust
/// Average True Range
pub fn atr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize
) -> Result<Vec<f64>>;
```

### Candlestick Patterns

```rust
/// Doji Pattern
pub fn doji(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    doji_pct: f64
) -> Result<Vec<i32>>;

/// Hammer Pattern
pub fn hammer(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64]
) -> Result<Vec<i32>>;

/// Engulfing Pattern
pub fn engulfing(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64]
) -> Result<Vec<i32>>;
```

### Chart Patterns

```rust
/// Double Top Detection
pub fn double_top(
    high: &[f64],
    lookback: usize,
    tolerance: f64
) -> Result<Vec<usize>>;

/// Head and Shoulders Detection
pub fn head_shoulders(
    high: &[f64],
    lookback: usize,
    tolerance: f64
) -> Result<Vec<usize>>;
```

### Streaming API (Incremental)

The `streaming` module provides O(1) per-bar indicator updates via the
[`StreamingIndicator`](https://docs.rs/finkit/latest/finkit/streaming/trait.StreamingIndicator.html)
trait. Use this path for live data feeds where re-scanning full history is impractical.

#### Core Traits

```rust
use finkit::streaming::{Ohlcv, OhlcvBar, StreamingIndicator, IndicatorMeta};

/// OHLCV bar access
pub trait Ohlcv {
    fn open(&self) -> f64;
    fn high(&self) -> f64;
    fn low(&self) -> f64;
    fn close(&self) -> f64;
    fn volume(&self) -> f64;
}

/// O(1) incremental update
pub trait StreamingIndicator<Input = f64, Output = f64> {
    fn next(&mut self, input: Input) -> Output;
    fn reset(&mut self);
    fn is_ready(&self) -> bool;
    fn count(&self) -> usize;
}

/// Machine-readable metadata
pub trait IndicatorMeta {
    fn name() -> &'static str;
    fn category() -> &'static str;
    fn description() -> &'static str;
    fn warm_up_period(&self) -> usize;
}
```

#### Available Streaming Indicators

| Struct | Input | Output | Module |
|--------|-------|--------|--------|
| `StreamingSma` | `f64` | `f64` | `streaming::indicators` |
| `StreamingEma` | `f64` | `f64` | `streaming::indicators` |
| `StreamingRsi` | `f64` | `f64` | `streaming::indicators` |
| `StreamingAtr` | `&dyn Ohlcv` | `f64` | `streaming::indicators` |
| `StreamingBoll` | `f64` | `BollOutput` | `streaming::indicators` |
| `StreamingMacd` | `f64` | `MacdOutput` | `streaming::indicators` |

```rust
use finkit::streaming::{StreamingIndicator, OhlcvBar};
use finkit::streaming::indicators::{StreamingSma, StreamingMacd, MacdOutput};

// Simple moving average — feed close prices one at a time
let mut sma = StreamingSma::new(3);
assert!(sma.next(1.0).is_nan());   // warming up
assert!(sma.next(2.0).is_nan());
assert_eq!(sma.next(3.0), 2.0);    // ready
assert!(sma.is_ready());

// MACD — returns structured output per bar
let mut macd = StreamingMacd::new(12, 26, 9);
let out: MacdOutput = macd.next(100.0);
// out.macd, out.signal, out.histogram

// OHLCV bar input for ATR
use finkit::streaming::indicators::StreamingAtr;
let bar = OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 1000.0);
let mut atr = StreamingAtr::new(14);
let val = atr.next(&bar);
```

#### Output Structs

```rust
pub struct BollOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

pub struct MacdOutput {
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}
```

### Indicator Registry API

The registry module exposes static metadata for all supported indicators,
enabling discovery, documentation generation, and JSON export.

```rust
use finkit::streaming::{
    all_indicators, registry_document,
    IndicatorInfo, ParamInfo, RegistryDocument,
};

/// Returns metadata for every registered indicator
pub fn all_indicators() -> Vec<IndicatorInfo>;

/// Builds the full registry document for JSON serialization
pub fn registry_document() -> RegistryDocument;
```

#### Data Types

```rust
pub struct IndicatorInfo {
    pub name: &'static str,
    pub category: &'static str,       // "overlap", "momentum", "volume", etc.
    pub description: &'static str,
    pub params: &'static [ParamInfo],
    pub convergence: usize,           // warm-up bars before valid output
}

pub struct ParamInfo {
    pub name: &'static str,
    pub param_type: &'static str,     // "usize", "f64", "str"
    pub default: &'static str,
    pub description: &'static str,
}

pub struct RegistryDocument {
    pub version: &'static str,
    pub generated_at: Option<&'static str>,
    pub indicators: Vec<IndicatorInfo>,
}
```

#### JSON Export

The canonical JSON snapshot lives at `docs/indicator_registry.json` and is
validated against `registry_document()` in core tests:

```rust
use finkit::streaming::registry_document;
use serde_json;

let json = serde_json::to_string_pretty(&registry_document()).unwrap();
// Write to docs/indicator_registry.json or serve via HTTP
```

Valid category slugs: `overlap`, `momentum`, `volume`, `volatility`, `price_transform`.

## Python API

### Installation

```bash
pip install finkit
```

### Functions

```python
import finkit as ta
import numpy as np

# Overlap Studies
def sma(close: np.ndarray, timeperiod: int = 14) -> np.ndarray: ...
def ema(close: np.ndarray, timeperiod: int = 14) -> np.ndarray: ...
def bollinger_bands(
    close: np.ndarray,
    timeperiod: int = 20,
    nbdevup: float = 2.0,
    nbdevdn: float = 2.0
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]: ...

# Momentum
def rsi(close: np.ndarray, timeperiod: int = 14) -> np.ndarray: ...
def macd(
    close: np.ndarray,
    fastperiod: int = 12,
    slowperiod: int = 26,
    signalperiod: int = 9
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]: ...
def stoch(
    high: np.ndarray,
    low: np.ndarray,
    close: np.ndarray,
    fastk_period: int = 5,
    slowk_period: int = 3,
    slowd_period: int = 3
) -> Tuple[np.ndarray, np.ndarray]: ...

# Volatility
def atr(
    high: np.ndarray,
    low: np.ndarray,
    close: np.ndarray,
    timeperiod: int = 14
) -> np.ndarray: ...

# Volume
def obv(close: np.ndarray, volume: np.ndarray) -> np.ndarray: ...

# Pattern Recognition
def cdl_doji(
    open: np.ndarray,
    high: np.ndarray,
    low: np.ndarray,
    close: np.ndarray,
    doji_pct: float = 0.1
) -> np.ndarray: ...

def detect_double_top(
    high: np.ndarray,
    lookback: int = 20,
    tolerance: float = 0.03
) -> np.ndarray: ...
```

### Complete Example

```python
import finkit as ta
import numpy as np
import pandas as pd

# Generate sample data
np.random.seed(42)
close = np.cumsum(np.random.randn(100)) + 100
high = close + np.random.uniform(0.5, 2.0, 100)
low = close - np.random.uniform(0.5, 2.0, 100)
volume = np.random.uniform(1000, 5000, 100)

# Calculate indicators
sma_20 = ta.sma(close, timeperiod=20)
rsi_14 = ta.rsi(close, timeperiod=14)
macd_line, signal, histogram = ta.macd(close)
upper, middle, lower = ta.bollinger_bands(close)

# Pattern recognition
doji_signals = ta.cdl_doji(open, high, low, close)
double_tops = ta.detect_double_top(high)
```

## Node.js API

### Installation

```bash
npm install finkit
```

### TypeScript Definitions

```typescript
export interface MacdResult {
  macd: number[];
  signal: number[];
  hist: number[];
}

export interface StochResult {
  k: number[];
  d: number[];
}

export interface BbandsResult {
  upper: number[];
  middle: number[];
  lower: number[];
}

// Overlap Studies
export function sma(close: number[], timeperiod?: number): number[];
export function ema(close: number[], timeperiod?: number): number[];
export function bollinger_bands(
  close: number[],
  timeperiod?: number,
  nbdevup?: number,
  nbdevdn?: number
): BbandsResult;

// Momentum
export function rsi(close: number[], timeperiod?: number): number[];
export function macd(
  close: number[],
  fastperiod?: number,
  slowperiod?: number,
  signalperiod?: number
): MacdResult;
export function stoch(
  high: number[],
  low: number[],
  close: number[],
  fastk_period?: number,
  slowk_period?: number,
  slowd_period?: number
): StochResult;

// Volatility
export function atr(
  high: number[],
  low: number[],
  close: number[],
  timeperiod?: number
): number[];

// Volume
export function obv(close: number[], volume: number[]): number[];

// Pattern Recognition
export function cdl_doji(
  open: number[],
  high: number[],
  low: number[],
  close: number[],
  doji_pct?: number
): number[];

export function detect_double_top(
  high: number[],
  lookback?: number,
  tolerance?: number
): number[];
```

### Complete Example

```typescript
import {
  sma, ema, rsi, macd, bollinger_bands,
  stoch, atr, obv,
  cdl_doji, cdl_hammer, detect_double_top
} from 'finkit';

const close = Array.from({ length: 100 }, (_, i) => 100 + i + Math.random());
const high = close.map(x => x + Math.random() * 2);
const low = close.map(x => x - Math.random() * 2);
const volume = Array.from({ length: 100 }, () => Math.random() * 4000 + 1000);

const smaResult = sma(close, 20);
const rsiResult = rsi(close, 14);
const macdResult = macd(close, 12, 26, 9);
const bbandsResult = bollinger_bands(close);
const atrResult = atr(high, low, close, 14);

const doji = cdl_doji(close, high, low, close);
const doubleTops = detect_double_top(high);
```

## Java API

### Maven Dependency

```xml
<dependency>
    <groupId>com.finkit</groupId>
    <artifactId>finkit</artifactId>
    <version>0.1.4</version>
</dependency>
```

### Classes

```java
package com.finkit;

public class Indicators {
    // Overlap Studies
    public static native double[] sma(double[] close, int timeperiod);
    public static native double[] ema(double[] close, int timeperiod);
    public static native BbandsResult bbands(double[] close, int timeperiod, double nbdevup, double nbdevdn);

    // Momentum
    public static native double[] rsi(double[] close, int timeperiod);
    public static native MacdResult macd(double[] close, int fastperiod, int slowperiod, int signalperiod);
    public static native StochResult stoch(double[] high, double[] low, double[] close, int fastk_period, int slowk_period, int slowd_period);

    // Volatility
    public static native double[] atr(double[] high, double[] low, double[] close, int timeperiod);

    // Volume
    public static native double[] obv(double[] close, double[] volume);

    // Pattern Recognition
    public static native int[] cdlDoji(double[] open, double[] high, double[] low, double[] close, double dojiPct);
    public static native int[] detectDoubleTop(double[] high, int lookback, double tolerance);
}

public class MacdResult {
    public double[] macd;
    public double[] signal;
    public double[] hist;
}

public class BbandsResult {
    public double[] upper;
    public double[] middle;
    public double[] lower;
}

public class StochResult {
    public double[] k;
    public double[] d;
}
```

### Complete Example

```java
import com.finkit.Indicators;
import com.finkit.MacdResult;

public class Example {
    public static void main(String[] args) {
        double[] close = new double[100];
        for (int i = 0; i < 100; i++) {
            close[i] = 100 + i + Math.random();
        }

        double[] sma20 = Indicators.sma(close, 20);
        double[] rsi14 = Indicators.rsi(close, 14);
        MacdResult macd = Indicators.macd(close, 12, 26, 9);

        System.out.println("SMA length: " + sma20.length);
        System.out.println("RSI length: " + rsi14.length);
        System.out.println("MACD length: " + macd.macd.length);
    }
}
```

## Go API

### Installation

```bash
go get github.com/coeasy/finkit
```

### Functions

```go
package ta

// Overlap Studies
func SMA(close []float64, timeperiod int) ([]float64, error)
func EMA(close []float64, timeperiod int) ([]float64, error)
func BBands(close []float64, timeperiod int, nbdevup, nbdevdn float64) (upper, middle, lower []float64, err error)

// Momentum
func RSI(close []float64, timeperiod int) ([]float64, error)
func MACD(close []float64, fastperiod, slowperiod, signalperiod int) (macd, signal, hist []float64, err error)
func Stoch(high, low, close []float64, fastk_period, slowk_period, slowd_period int) (k, d []float64, err error)

// Volatility
func ATR(high, low, close []float64, timeperiod int) ([]float64, error)

// Volume
func OBV(close, volume []float64) ([]float64, error)

// Pattern Recognition
func CDLDoji(open, high, low, close []float64, doji_pct float64) ([]int32, error)
func DetectDoubleTop(high []float64, lookback int, tolerance float64) ([]int, error)
```

### Complete Example

```go
package main

import (
    "fmt"
    "github.com/coeasy/finkit/go/ta"
)

func main() {
    close := make([]float64, 100)
    for i := 0; i < 100; i++ {
        close[i] = float64(i + 1)
    }

    sma, err := ta.SMA(close, 20)
    if err != nil {
        panic(err)
    }

    rsi, err := ta.RSI(close, 14)
    if err != nil {
        panic(err)
    }

    macd, signal, hist, err := ta.MACD(close, 12, 26, 9)
    if err != nil {
        panic(err)
    }

    fmt.Printf("SMA length: %d\n", len(sma))
    fmt.Printf("RSI length: %d\n", len(rsi))
    fmt.Printf("MACD length: %d\n", len(macd))
}
```

## .NET API

### NuGet Package

```bash
dotnet add package finkit
```

### Classes

```csharp
namespace Finkit;

public class Indicators
{
    // Overlap Studies
    public static double[] SMA(double[] close, int timeperiod = 14);
    public static double[] EMA(double[] close, int timeperiod = 14);
    public static BbandsResult BBands(double[] close, int timeperiod = 20, double nbdevup = 2.0, double nbdevdn = 2.0);

    // Momentum
    public static double[] RSI(double[] close, int timeperiod = 14);
    public static MacdResult MACD(double[] close, int fastperiod = 12, int slowperiod = 26, int signalperiod = 9);

    // Volatility
    public static double[] ATR(double[] high, double[] low, double[] close, int timeperiod = 14);

    // Volume
    public static double[] OBV(double[] close, double[] volume);

    // Pattern Recognition
    public static int[] CDLDoji(double[] open, double[] high, double[] low, double[] close, double dojiPct = 0.1);
    public static int[] DetectDoubleTop(double[] high, int lookback = 20, double tolerance = 0.03);
}

public class MacdResult
{
    public double[] Macd { get; set; }
    public double[] Signal { get; set; }
    public double[] Hist { get; set; }
}

public class BbandsResult
{
    public double[] Upper { get; set; }
    public double[] Middle { get; set; }
    public double[] Lower { get; set; }
}
```

### Complete Example

```csharp
using System;
using System.Linq;
using Finkit;

class Program
{
    static void Main()
    {
        var close = Enumerable.Range(0, 100)
            .Select(i => 100.0 + i + new Random().NextDouble())
            .ToArray();

        var sma20 = Indicators.SMA(close, 20);
        var rsi14 = Indicators.RSI(close, 14);
        var macd = Indicators.MACD(close);

        Console.WriteLine($"SMA length: {sma20.Length}");
        Console.WriteLine($"RSI length: {rsi14.Length}");
        Console.WriteLine($"MACD length: {macd.Macd.Length}");
    }
}
```

## WebAssembly API

### Installation

```bash
npm install finkit-wasm
```

### Functions

```typescript
// Initialize WASM module
export default function init(): Promise<void>;

// Overlap Studies
export function sma(close: number[], timeperiod?: number): number[];
export function ema(close: number[], timeperiod?: number): number[];

// Momentum
export function rsi(close: number[], timeperiod?: number): number[];
export function macd(
  close: number[],
  fastperiod?: number,
  slowperiod?: number,
  signalperiod?: number
): { macd: number[]; signal: number[]; hist: number[] };

// Volatility
export function atr(
  high: number[],
  low: number[],
  close: number[],
  timeperiod?: number
): number[];

// Volume
export function obv(close: number[], volume: number[]): number[];
```

### Complete Example

```typescript
import init, { sma, rsi, macd } from 'finkit-wasm';

async function main() {
  await init();

  const close = Array.from({ length: 100 }, (_, i) => i + 1);
  const smaResult = sma(close, 20);
  const rsiResult = rsi(close, 14);
  const macdResult = macd(close, 12, 26, 9);

  console.log(`SMA length: ${smaResult.length}`);
  console.log(`RSI length: ${rsiResult.length}`);
  console.log(`MACD length: ${macdResult.macd.length}`);
}

main();
```

## CLI API

### Installation

```bash
cargo install finkit
```

### Commands

```bash
# Calculate indicators
finkit sma --input data.csv --period 14 --output sma.csv
finkit ema --input data.csv --period 14
finkit rsi --input data.csv --period 14
finkit macd --input data.csv --fast 12 --slow 26 --signal 9
finkit bbands --input data.csv --period 20 --nbdevup 2.0 --nbdevdn 2.0
finkit atr --input data.csv --period 14

# Detect patterns
finkit patterns --input data.csv --format json
finkit candlestick --input data.csv
finkit chart-patterns --input data.csv

# Export options
finkit rsi --input data.csv --output rsi.csv --format csv
finkit rsi --input data.csv --output rsi.json --format json
```

### CSV Input Format

```csv
timestamp,open,high,low,close,volume
2024-01-01,100.0,101.0,99.0,100.5,1000
2024-01-02,100.5,102.0,100.0,101.0,1200
...
```

### Output Format

```json
{
  "indicator": "RSI",
  "parameters": { "timeperiod": 14 },
  "data": [
    { "timestamp": "2024-01-01", "value": null },
    { "timestamp": "2024-01-15", "value": 52.34 },
    ...
  ]
}
```

## Error Types

### Rust

```rust
#[derive(Debug, Error)]
pub enum TaError {
    #[error("Invalid period: {0}")]
    InvalidPeriod(usize),

    #[error("Insufficient data: need {needed}, got {actual}")]
    InsufficientData { needed: usize, actual: usize },

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Computation error: {0}")]
    ComputationError(String),
}
```

### Python

```python
class TaLibError(Exception):
    """Base exception for all TA-Lib errors."""
    pass

class InvalidPeriodError(TaLibError):
    """Raised when period parameter is invalid."""
    pass

class InsufficientDataError(TaLibError):
    """Raised when input data is too short."""
    pass

class InvalidParametersError(TaLibError):
    """Raised when parameters are out of valid range."""
    pass
```

### Node.js

```typescript
export class TaLibError extends Error {
  constructor(
    message: string,
    public code: string,
    public details?: Record<string, any>
  ) {
    super(message);
    this.name = 'TaLibError';
  }
}
```
