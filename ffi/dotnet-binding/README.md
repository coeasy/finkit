# AlphaTA .NET Binding

High-performance technical analysis indicators for .NET, powered by Rust via P/Invoke.

## Features

- **40+ Technical Indicators**: SMA, EMA, RSI, MACD, Bollinger Bands, Stochastic, ADX, ATR, Hilbert Transform, and more
- **Blazing Fast**: Native Rust performance with zero GC pressure from unmanaged code
- **Cross-Platform**: Supports Windows, Linux, and macOS
- **Full .NET Support**: .NET 6, .NET 8, and .NET Standard 2.0
- **Safe API**: Memory-safe with proper P/Invoke marshaling

## Installation

### Prerequisites

- .NET 6.0 SDK or later
- Rust toolchain (for building the native library)

### Building

```bash
# Build the native Rust library
cd ffi/dotnet-binding
cargo build --release

# Copy the native library to the output directory
# Windows:
cp target/release/alpha_ta_dotnet.dll src/AlphaTA/bin/Release/native/
# Linux/macOS:
cp target/release/libalpha_ta_dotnet.so src/AlphaTA/bin/Release/native/
cp target/release/libalpha_ta_dotnet.dylib src/AlphaTA/bin/Release/native/

# Build the .NET library
cd src/AlphaTA
dotnet build -c Release
```

## Usage

```csharp
using AlphaTA;

// Simple Moving Average
double[] prices = { 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0 };
double[] sma = Indicators.Sma(prices, 3);
// sma[2] = 2.0, sma[3] = 3.0, sma[4] = 4.0, ...

// MACD
var macd = Indicators.Macd(prices, 12, 26, 9);
double[] macdLine = macd.Macd;
double[] signalLine = macd.Signal;
double[] histogram = macd.Hist;

// Bollinger Bands
var bbands = Indicators.Bbands(prices, 10, 2.0, 2.0);
double[] upper = bbands.Upper;
double[] middle = bbands.Middle;
double[] lower = bbands.Lower;

// RSI
double[] rsi = Indicators.Rsi(prices, 14);
```

## Available Indicators

### Overlap Studies
- `Sma` - Simple Moving Average
- `Ema` - Exponential Moving Average
- `Wma` - Weighted Moving Average
- `Dema` - Double Exponential Moving Average
- `Tema` - Triple Exponential Moving Average
- `Kama` - Kaufman Adaptive Moving Average
- `T3` - T3 Moving Average
- `Bbands` - Bollinger Bands

### Momentum Indicators
- `Rsi` - Relative Strength Index
- `Macd` - Moving Average Convergence Divergence
- `Stoch` - Stochastic Oscillator
- `Adx` - Average Directional Index
- `Aroon` - Aroon Indicator
- `Cci` - Commodity Channel Index
- `Mom` - Momentum
- `Roc` - Rate of Change
- `Willr` - Williams %R

### Volume Indicators
- `Obv` - On Balance Volume
- `Ad` - Accumulation/Distribution Line
- `AdOsc` - Chaikin A/D Oscillator

### Volatility Indicators
- `Atr` - Average True Range
- `Natr` - Normalized ATR
- `Trange` - True Range

### Hilbert Transform Indicators
- `HtDcPeriod` - Dominant Cycle Period
- `HtDcPhase` - Dominant Cycle Phase
- `HtPhasor` - Phasor Components
- `HtSine` - Sine Wave
- `HtTrendMode` - Trend vs Cycle Mode
- `HtTrendLine` - Instantaneous Trendline

### Statistics Indicators
- `ZScore` - Z-Score
- `Beta` - Beta Coefficient
- `Correlation` - Pearson Correlation
- `StdDev` - Standard Deviation
- `LinearReg` - Linear Regression
- `Tsf` - Time Series Forecast

### Adaptive Moving Averages
- `Mama` - MESA Adaptive Moving Average

## Error Handling

All indicator methods throw `InvalidOperationException` if the calculation fails. Error codes from the native library:
- `0` - Success
- `-1` - Invalid parameters (null input, invalid length/period)
- `-2` - Calculation error (insufficient data, etc.)

## Native Library Loading

The library automatically loads the native Rust library from:
1. `{app_directory}/native/{platform}/{architecture}/`
2. Standard search paths (application directory, assembly directory)

For NuGet package distribution, include the native libraries with the appropriate RID structure.

## Memory Management

Several native APIs return heap-allocated C strings (e.g. `ta_get_version`, `ta_get_last_error`,
indicator name lists, JSON serialised results). The .NET side **must** pair every successful
string-returning call with `Marshal.FreeHGlobal` (or with the dedicated `ta_free_cstring`
helper) to avoid memory leaks.

### `ta_free_cstring` contract

- C signature:
  ```c
  void ta_free_cstring(const char* s);
  ```
  Implementation: `unsafe { drop(CString::from_raw(s as *mut c_char)) }` — i.e. it
  reclaims the exact allocation produced by the matching `ta_*` call.
- The function is exported with `#[no_mangle] pub extern "C"` from
  `ffi/dotnet-binding/src/lib.rs` and is safe to call with a `null` pointer (no-op).
- The .NET side **must**:
  1. Receive the `*mut c_char` (or `IntPtr`) from a `ta_*` call.
  2. Marshal it to a managed `string` (e.g. via `Marshal.PtrToStringAnsi`) **once**.
  3. Call `ta_free_cstring` (directly or via the bundled `AlphaTA.Native.FreeCString` wrapper) to release the native buffer.
- Do **not** call `Marshal.FreeHGlobal` on a pointer obtained from `ta_*` unless you have
  verified that the native function returned a pointer allocated by `malloc` (most `ta_*`
  functions use Rust's allocator; mixing them with the CRT allocator is **undefined
  behaviour**). Prefer `ta_free_cstring` for safety.
- The same string pointer must never be freed twice (double-free ⇒ UB).

### Example

```csharp
using AlphaTA;
using System.Runtime.InteropServices;

IntPtr raw = NativeBindings.ta_get_version();   // returns *mut c_char
string version = Marshal.PtrToStringAnsi(raw);  // single marshal copy
NativeBindings.ta_free_cstring(raw);             // free native allocation
Console.WriteLine($"AlphaTA {version}");
```

## License

MIT
