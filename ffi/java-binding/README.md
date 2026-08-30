# Finkit Java Bindings

High-performance technical analysis indicators for Java, powered by a Rust core via JNI.

## Features

- **100+ indicators** across all major categories
- **Zero-copy JNI** for maximum performance
- **Multi-platform**: Windows, Linux, macOS (x86_64 & aarch64)
- **Java 8-21** compatible
- **Complete Javadoc** documentation
- **Native library auto-loading** (no manual LD_LIBRARY_PATH setup)

## Indicators

### Overlap Studies
| Indicator | Description |
|-----------|-------------|
| SMA | Simple Moving Average |
| EMA | Exponential Moving Average |
| WMA | Weighted Moving Average |
| DEMA | Double Exponential Moving Average |
| TEMA | Triple Exponential Moving Average |
| KAMA | Kaufman's Adaptive Moving Average |
| T3 | Triple EMA with variable smoothing |
| MAMA | MESA Adaptive Moving Average |
| BBANDS | Bollinger Bands |
| MIDPOINT | MidPoint over period |
| MIDPRICE | MidPrice over period |
| SAR | Parabolic SAR |

### Momentum Indicators
| Indicator | Description |
|-----------|-------------|
| RSI | Relative Strength Index |
| MACD | Moving Average Convergence Divergence |
| STOCH | Stochastic Oscillator |
| ADX | Average Directional Index |
| AROON | Aroon Up/Down |
| CCI | Commodity Channel Index |
| CMO | Chande Momentum Oscillator |
| DX | Directional Movement Index |
| MOM | Momentum |
| ROC | Rate of Change |
| WILLR | Williams %R |
| APO | Absolute Price Oscillator |
| BOP | Balance of Power |
| MFI | Money Flow Index |
| PLUS_DI | Plus Directional Indicator |
| MINUS_DI | Minus Directional Indicator |
| TRIX | Triple Exponential Average |

### Volume Indicators
| Indicator | Description |
|-----------|-------------|
| OBV | On Balance Volume |
| AD | Accumulation/Distribution Line |
| ADOSC | AD Oscillator |

### Volatility Indicators
| Indicator | Description |
|-----------|-------------|
| ATR | Average True Range |
| NATR | Normalized ATR |
| TRANGE | True Range |

### Price Transforms
| Indicator | Description |
|-----------|-------------|
| AVGPRICE | Average Price |
| MEDPRICE | Median Price |
| TYPPRICE | Typical Price |
| WCLPRICE | Weighted Close Price |

### Cycle Indicators (Hilbert Transform)
| Indicator | Description |
|-----------|-------------|
| HT_DCPERIOD | Dominant Cycle Period |
| HT_DCPHASE | Dominant Cycle Phase |
| HT_PHASOR | Phasor Components |
| HT_SINE | Sine Wave |
| HT_TRENDMODE | Trend vs Cycle Mode |
| HT_TRENDLINE | Instantaneous Trendline |

### Statistical Indicators
| Indicator | Description |
|-----------|-------------|
| ZSCORE | Z-Score |
| PERCENT_RANK | Percent Rank |
| BETA | Beta |
| CORRELATION | Pearson Correlation |
| STDDEV | Standard Deviation |
| LINEAR_REG | Linear Regression |
| TSF | Time Series Forecast |

### Candlestick Patterns (60+ patterns)
- **Single**: Doji, Marubozu, Hammer, Hanging Man, Shooting Star, Spinning Top, etc.
- **Dual**: Engulfing, Harami, Harami Cross, Piercing, Dark Cloud Cover, Tweezer, etc.
- **Triple**: Morning Star, Evening Star, Three White Soldiers, Three Black Crows, etc.
- **Complex**: Abandoned Baby, Kicking, Concealing Baby Swallow, Breakaway, etc.

### Chart Patterns (15+ patterns)
- **Reversal**: Head & Shoulders (Top/Bottom), Double/Triple Top/Bottom
- **Triangle**: Ascending, Descending, Symmetrical
- **Wedge**: Rising, Falling
- **Continuation**: Pennant, Flag, Rectangle

## Installation

### Maven

```xml
<dependency>
    <groupId>com.finkit</groupId>
    <artifactId>finkit</artifactId>
    <version>1.0.0</version>
</dependency>
```

### Gradle

```groovy
implementation 'com.finkit:finkit:1.0.0'
```

## Quick Start

```java
import com.finkit.*;

public class Example {
    public static void main(String[] args) {
        // Simple Moving Average
        double[] prices = {100.0, 101.0, 102.0, 103.0, 104.0, 105.0};
        double[] sma = Indicators.sma(prices, 3);

        // MACD
        MacdResult macd = new MacdResult();
        Indicators.macd(prices, 12, 26, 9, macd);
        // macd.macd, macd.signal, macd.hist

        // Bollinger Bands
        BbandsResult bbands = new BbandsResult();
        Indicators.bbands(prices, 20, 2.0, 2.0, bbands);
        // bbands.upper, bbands.middle, bbands.lower

        // RSI
        double[] rsi = Indicators.rsi(prices, 14);

        // Stochastic
        StochResult stoch = new StochResult();
        Indicators.stoch(high, low, close, 14, 3, 3, stoch);
        // stoch.k, stoch.d

        // Candlestick Pattern
        int[] hammer = Patterns.cdlHammer(open, high, low, close);
        for (int i = 0; i < hammer.length; i++) {
            if (hammer[i] == 100) {
                System.out.println("Bullish hammer at bar " + i);
            } else if (hammer[i] == -100) {
                System.out.println("Bearish hammer at bar " + i);
            }
        }

        // Chart Pattern
        int[] hs = ChartPatterns.detectHeadShouldersTop(high, 10, 1.1);
        for (int i = 0; i < hs.length; i++) {
            if (hs[i] == -1) {
                System.out.println("H&S Top at bar " + i);
            }
        }
    }
}
```

## Native Library Loading

The native library is automatically loaded when any indicator class is first used. Supported platforms:

| OS | Architecture | Library File |
|----|-------------|--------------|
| Windows | x86_64 | `finkit_java.dll` |
| Linux | x86_64 | `libfinkit_java.so` |
| Linux | aarch64 | `libfinkit_java.so` |
| macOS | x86_64 | `libfinkit_java.dylib` |
| macOS | aarch64 | `libfinkit_java.dylib` |

If the native library is not bundled with the JAR, set the library path:

```bash
java -Djava.library.path=/path/to/native/lib -jar your-app.jar
```

Or programmatically:

```java
System.setProperty("java.library.path", "/path/to/native/lib");
```

## Building from Source

### Prerequisites
- Rust 1.70+
- Java 8+ (JDK)
- Maven 3.6+

### Build native library

```bash
cd ffi/java-binding
cargo build --release
```

### Build Java JAR

```bash
cd ffi/java-binding
mvn clean package
```

### Run tests

```bash
# Rust tests
cargo test

# Java tests (if any)
mvn test
```

## API Reference

Full Javadoc is available at:
- [Indicators](https://javadoc.io/doc/com.finkit/finkit/latest/com.finkit/Indicators.html)
- [Patterns](https://javadoc.io/doc/com.finkit/finkit/latest/com.finkit/Patterns.html)
- [ChartPatterns](https://javadoc.io/doc/com.finkit/finkit/latest/com.finkit/ChartPatterns.html)

## Memory Management

Several JNI methods return a `jstring` (e.g. `getVersion`, `getLastError`, indicator
name lists, JSON-serialised results). Each returned `jstring` is a **JNI local reference**
allocated by the native side via `env.new_string(...).into_raw()`. Local refs are
scoped to the calling thread's JNI frame and **must be explicitly released** by the
caller to avoid the JVM's local reference table growing unbounded (which manifests as
`JNI ERROR (app bug): local reference table overflow` on long-running batches).

### `freeJString` contract

- JNI signature:
  ```
  Java_com_finkit_Indicators_freeJString(JNIEnv* env, jclass, jstring ref)
  ```
  Implementation: `env->DeleteLocalRef(ref)`. The function is exported from
  `ffi/java-binding/src/lib.rs` and is safe to call with a `null` `jstring` (no-op).
- The Java side **must**:
  1. Receive the `String` (or `jstring` reference) from a `ta_*` call.
  2. Copy the contents into a managed `String` (e.g. via `new String(jstring.getBytes(UTF_8), UTF_8)`) once.
  3. Call `Indicators.freeJString(jstringRef)` to release the JNI local reference. The
     `jstring` reference passed in is the **same reference** returned by the native
     call, **not** the managed `String` you copied to.
- The same `jstring` ref must never be freed twice (double-free ⇒ JNI may abort the
  JVM or silently corrupt the local reference table).
- The native side no longer holds the string after returning it — the caller's
  release is the only release.

### Example

```java
import com.finkit.Indicators;

long jstrRef = Indicators.getVersion();          // raw jstring ref (long)
String version = Indicators.fromJString(jstrRef); // copy to managed String
Indicators.freeJString(jstrRef);                  // release JNI local ref
System.out.println("Finkit " + version);
```

> **Why explicit release?** The previous design relied on the JVM's `PushLocalFrame` /
> `PopLocalFrame` cleanup at the end of the calling method. While that usually works,
> it leaks local refs in long-running batch loops (one ref per call) until the
> per-thread table overflows. The explicit `freeJString` contract eliminates this
> class of bug.

## License

MIT License. See [LICENSE](../../LICENSE) for details.
