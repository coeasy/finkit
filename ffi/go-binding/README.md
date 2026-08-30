# Finkit Go Binding

Go bindings for the finkit technical analysis library, using CGO for high-performance FFI.

## Requirements

- **Go**: 1.21 or later
- **Rust**: 1.70 or later
- **CGO**: Enabled (usually enabled by default)

## Build

```bash
# Build the Rust library
make build

# Or manually
cd ../..
cargo build --release -p finkit-go
```

The compiled library will be available in `../../target/release/`.

## Usage

Import the package in your Go code:

```go
import "github.com/coeasy/finkit/go/ta"
```

Example:

```go
package main

import (
    "fmt"
    "github.com/coeasy/finkit/go/ta"
)

func main() {
    prices := []float64{44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0}
    
    // Calculate RSI
    rsi, err := ta.Rsi(prices, 14)
    if err != nil {
        panic(err)
    }
    fmt.Println("RSI:", rsi)
    
    // Calculate MACD
    macd, err := ta.Macd(prices, 12, 26, 9)
    if err != nil {
        panic(err)
    }
    fmt.Println("MACD:", macd.Macd)
}
```

## Available Indicators

### Moving Averages
- `Sma` - Simple Moving Average
- `Ema` - Exponential Moving Average
- `Wma` - Weighted Moving Average
- `Dema` - Double Exponential Moving Average
- `Tema` - Triple Exponential Moving Average
- `Kama` - Kaufman Adaptive Moving Average
- `T3` - T3 Moving Average

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
- `Bbands` - Bollinger Bands

### Hilbert Transform Indicators
- `HtDcPeriod` - Dominant Cycle Period
- `HtDcPhase` - Dominant Cycle Phase
- `HtPhasor` - Phasor Components
- `HtSine` - Sine Wave
- `HtTrendMode` - Trend vs Cycle Mode
- `HtTrendLine` - Instantaneous Trendline

### Statistical Functions
- `ZScore` - Z-Score
- `Beta` - Beta Coefficient
- `Correlation` - Pearson Correlation
- `StdDev` - Standard Deviation
- `LinearReg` - Linear Regression
- `Tsf` - Time Series Forecast

## Test

```bash
make test
```

## Clean

```bash
make clean
```

## License

MIT OR Apache-2.0
