# Complete Indicator List

This document provides a comprehensive list of all technical analysis indicators available in Finkit, organized by category.

## Overlap Studies

Overlap indicators are plotted directly on the price chart and help identify trends and support/resistance levels.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| SMA | Simple Moving Average | `close, timeperiod` | Array | Unweighted arithmetic mean of closing prices |
| EMA | Exponential Moving Average | `close, timeperiod` | Array | Weighted average giving more weight to recent prices |
| WMA | Weighted Moving Average | `close, timeperiod` | Array | Linearly weighted moving average |
| DEMA | Double Exponential Moving Average | `close, timeperiod` | Array | Reduces lag by using double EMA smoothing |
| TEMA | Triple Exponential Moving Average | `close, timeperiod` | Array | Further reduces lag with triple EMA smoothing |
| KAMA | Kaufman Adaptive Moving Average | `close, timeperiod` | Array | Adapts to market noise using efficiency ratio |
| MAMA | MESA Adaptive Moving Average | `close, fastlimit, slowlimit` | Array, Array | Adaptive moving average using Hilbert transform |
| T3 | Triple Exponential Moving Average (T3) | `close, timeperiod, vfactor` | Array | Smooth TEMA with volume factor adjustment |
| BBANDS | Bollinger Bands | `close, timeperiod, nbdevup, nbdevdn, matype` | Array, Array, Array | Upper band, middle band, lower band using standard deviation |
| SAR | Parabolic SAR | `high, low, acceleration, maximum` | Array | Stop and Reverse points for trend following |
| HT_TRENDLINE | Hilbert Instantaneous Trendline | `close` | Array | Trendline using Hilbert transform |
| MIDPOINT | MidPoint over period | `close, timeperiod` | Array | (Highest high + Lowest low) / 2 over period |
| MIDPRICE | Midpoint Price over period | `high, low, timeperiod` | Array | (Highest high + Lowest low) / 2 using high/low |
| MAVP | Moving Average with Variable Period | `close, periods, minperiod, maxperiod, matype` | Array | Moving average with dynamically changing period |
| TRIMA | Triangular Moving Average | `close, timeperiod` | Array | Double-smoothed SMA for reduced noise |

## Momentum Indicators

Momentum indicators measure the speed and magnitude of price changes.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| RSI | Relative Strength Index | `close, timeperiod` | Array | Measures speed and change of price movements (0-100) |
| MACD | Moving Average Convergence/Divergence | `close, fastperiod, slowperiod, signalperiod` | Array, Array, Array | MACD line, signal line, histogram |
| STOCH | Stochastic | `high, low, close, fastk_period, slowk_period, slowk_matype, slowd_period, slowd_matype` | Array, Array | Slow %K and Slow %D lines |
| STOCHF | Stochastic Fast | `high, low, close, fastk_period, fastd_period, fastd_matype` | Array, Array | Fast %K and Fast %D lines |
| ADX | Average Directional Movement Index | `high, low, close, timeperiod` | Array | Strength of trend (0-100) |
| AROON | Aroon | `high, low, timeperiod` | Array, Array | Aroon Up and Aroon Down |
| AROONOSC | Aroon Oscillator | `high, low, timeperiod` | Array | Aroon Up - Aroon Down |
| CCI | Commodity Channel Index | `high, low, close, timeperiod` | Array | Identifies cyclical turns in commodity prices |
| CMO | Chande Momentum Oscillator | `close, timeperiod` | Array | Momentum oscillator (-100 to +100) |
| MOM | Momentum | `close, timeperiod` | Array | Difference between current and past price |
| ROC | Rate of Change | `close, timeperiod` | Array | Percentage rate of change |
| WILLR | Williams' %R | `high, low, close, timeperiod` | Array | Overbought/oversLevels indicator (-100 to 0) |
| APO | Absolute Price Oscillator | `close, fastperiod, slowperiod, matype` | Array | Difference between two moving averages |
| BOP | Balance Of Power | `open, high, low, close` | Array | Measures strength of buyers vs sellers |
| DX | Directional Movement Index | `high, low, close, timeperiod` | Array | Directional movement without trend strength |
| MFI | Money Flow Index | `high, low, close, volume, timeperiod` | Array | Volume-weighted RSI (0-100) |
| MINUS_DI | Minus Directional Indicator | `high, low, close, timeperiod` | Array | Negative directional movement |
| MINUS_DM | Minus Directional Movement | `high, low, timeperiod` | Array | Negative directional movement raw value |
| PLUS_DI | Plus Directional Indicator | `high, low, close, timeperiod` | Array | Positive directional movement |
| PLUS_DM | Plus Directional Movement | `high, low, timeperiod` | Array | Positive directional movement raw value |
| TRIX | 1-day Rate-Of-Change of Triple Smooth EMA | `close, timeperiod` | Array | Rate of change of triple EMA |
| ULTOSC | Ultimate Oscillator | `high, low, close, timeperiod1, timeperiod2, timeperiod3` | Array | Combines three timeframes of buying pressure |

## Volume Indicators

Volume indicators incorporate trading volume to confirm trends.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| AD | Chaikin A/D Line | `high, low, close, volume` | Array | Accumulation/Distribution line |
| ADOSC | Chaikin A/D Oscillator | `high, low, close, volume, fastperiod, slowperiod` | Array | Oscillator of A/D line |
| OBV | On Balance Volume | `close, volume` | Array | Cumulative volume based on price direction |
| CMF | Chaikin Money Flow | `high, low, close, volume, timeperiod` | Array | Measures money flow over specified period |

## Volatility Indicators

Volatility indicators measure the rate and magnitude of price fluctuations.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| ATR | Average True Range | `high, low, close, timeperiod` | Array | Average of true ranges over period |
| NATR | Normalized Average True Range | `high, low, close, timeperiod` | Array | ATR normalized by closing price (percentage) |
| TRANGE | True Range | `high, low, close` | Array | Greatest of: high-low, |high-prev_close|, |low-prev_close| |
| KAMA_VOLATILITY | KAMA Volatility | `close, timeperiod` | Array | Volatility measure using Kaufman adaptive method |

## Cycle Indicators

Cycle indicators use Hilbert transforms to identify market cycles.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| HT_DCPERIOD | Hilbert Dominant Cycle Period | `close` | Array | Dominant cycle period in bars |
| HT_DCPHASE | Hilbert Dominant Cycle Phase | `close` | Array | Dominant cycle phase angle |
| HT_PHASOR | Hilbert Phasor Components | `close` | Array, Array | In-phase and quadrature components |
| HT_SINE | Hilbert SineWave | `close` | Array, Array | Sine and lead sine waves |
| HT_TRENDMODE | Hilbert Trend vs Cycle Mode | `close` | Array | 1 = trend mode, 0 = cycle mode |
| HT_MEASUREMENT | Hilbert Measurement | `close` | Array | Combined Hilbert transform measurements |

## Price Transform

Price transforms convert OHLC data into alternative price representations.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| AVGPRICE | Average Price | `open, high, low, close` | Array | (Open + High + Low + Close) / 4 |
| MEDPRICE | Median Price | `high, low` | Array | (High + Low) / 2 |
| TYPPRICE | Typical Price | `high, low, close` | Array | (High + Low + Close) / 3 |
| WCLPRICE | Weighted Close Price | `high, low, close` | Array | (High + Low + Close*2) / 4 |
| MEDIPRICE | Median Price with OHLC | `open, high, low, close` | Array | Weighted median price calculation |

## Statistics

Statistical indicators provide measures of dispersion and correlation.

| Indicator | Full Name | Parameters | Output | Description |
|-----------|-----------|------------|--------|-------------|
| STDDEV | Standard Deviation | `close, timeperiod, nbdev` | Array | Standard deviation of closing prices |
| VAR | Variance | `close, timeperiod, nbdev` | Array | Variance of closing prices |
| LINEARREG | Linear Regression | `close, timeperiod` | Array | Linear regression line values |
| LINEARREG_ANGLE | Linear Regression Angle | `close, timeperiod` | Array | Angle of linear regression line in degrees |
| LINEARREG_INTERCEPT | Linear Regression Intercept | `close, timeperiod` | Array | Intercept of linear regression line |
| LINEARREG_SLOPE | Linear Regression Slope | `close, timeperiod` | Array | Slope of linear regression line |
| TSF | Time Series Forecast | `close, timeperiod` | Array | Extrapolated value of linear regression |
| ZSCORE | Z-Score | `close, timeperiod` | Array | Number of standard deviations from mean |
| CORREL | Pearson's Correlation Coefficient | `close_a, close_b, timeperiod` | Array | Correlation between two price series |

## Pattern Recognition

### Candlestick Patterns (60+)

Candlestick patterns return an array of integers: 100 for bullish, -100 for bearish, 0 for no pattern.

| Pattern | Full Name | Parameters | Description |
|---------|-----------|------------|-------------|
| CDL2CROWS | Two Crows | `open, high, low, close` | Bearish reversal pattern |
| CDL3BLACKCROWS | Three Black Crows | `open, high, low, close` | Bearish reversal with three long black candles |
| CDL3INSIDE | Three Inside Up/Down | `open, high, low, close` | Three-candle reversal pattern |
| CDL3OUTSIDE | Three Outside Up/Down | `open, high, low, close` | Confirmed engulfing pattern |
| CDL3STARSINSOUTH | Three Stars In The South | `open, high, low, close` | Rare bullish reversal |
| CDL3WHITESOLDIERS | Three Advancing White Soldiers | `open, high, low, close` | Bullish reversal with three white candles |
| CDLABANDONEDBABY | Abandoned Baby | `open, high, low, close, penetration` | Rare reversal with doji gap |
| CDLADVANCEBLOCK | Advance Block | `open, high, low, close` | Bearish pattern in uptrend |
| CDLBELTHOLD | Belt-hold | `open, high, low, close` | Single candle with no shadows on one side |
| CDLBREAKAWAY | Breakaway | `open, high, low, close` | Reversal pattern with five candles |
| CDLCLOSINGMARUBOZU | Closing Marubozu | `open, high, low, close` | Candle with closing at extreme |
| CDLCONCEALBABYSWALL | Concealing Baby Swallow | `open, high, low, close` | Bullish reversal with four candles |
| CDLCOUNTERATTACK | Counterattack | `open, high, low, close` | Reversal at support/resistance |
| CDLDARKCLOUDCOVER | Dark Cloud Cover | `open, high, low, close, penetration` | Bearish reversal pattern |
| CDLDOJI | Doji | `open, high, low, close, doji_pct` | Open and close are virtually equal |
| CDLDOJISTAR | Doji Star | `open, high, low, close, doji_pct` | Doji following a trend |
| CDLDRAGONFLYDOJI | Dragonfly Doji | `open, high, low, close, doji_pct` | Doji with long lower shadow |
| CDLENGULFING | Engulfing Pattern | `open, high, low, close` | Second candle engulfs first |
| CDLEVENINGDOJISTAR | Evening Doji Star | `open, high, low, close, penetration` | Bearish three-candle pattern |
| CDLEVENINGSTAR | Evening Star | `open, high, low, close, penetration` | Bearish reversal pattern |
| CDLGAPSIDESIDEWHITE | Up/Down-gap side-by-side white lines | `open, high, low, close` | Continuation pattern |
| CDLGRAVESTONEDOJI | Gravestone Doji | `open, high, low, close, doji_pct` | Doji with long upper shadow |
| CDLHAMMER | Hammer | `open, high, low, close` | Bullish reversal with small body |
| CDLHANGINGMAN | Hanging Man | `open, high, low, close` | Bearish reversal with small body |
| CDLHARAMI | Harami Pattern | `open, high, low, close` | Inside day pattern |
| CDLHARAMICROSS | Harami Cross Pattern | `open, high, low, close, doji_pct` | Inside doji pattern |
| CDLHIGHWAVE | High-Wave Candle | `open, high, low, close` | Long upper and lower shadows |
| CDLHIKKAKE | Hikkake Pattern | `open, high, low, close` | Trap and reversal pattern |
| CDLHIKKAKEMOD | Modified Hikkake Pattern | `open, high, low, close` | Enhanced hikkake pattern |
| CDLHOMINGPIGEON | Homing Pigeon | `open, high, low, close` | Bullish reversal pattern |
| CDLIDENTICAL3CROWS | Identical Three Crows | `open, high, low, close` | Three similar black candles |
| CDLINNECK | In-Neck Pattern | `open, high, low, close` | Continuation pattern |
| CDLINVERTEDHAMMER | Inverted Hammer | `open, high, low, close` | Bullish reversal with upper shadow |
| CDLKICKING | Kicking | `open, high, low, close` | Strong reversal with marubozu |
| CDLKICKINGBYLENGTH | Kicking (by length) | `open, high, low, close` | Kicking by longer marubozu |
| CDLLADDERBOTTOM | Ladder Bottom | `open, high, low, close` | Bullish reversal with five candles |
| CDLLONGLEGGEDDOJI | Long Legged Doji | `open, high, low, close, doji_pct` | Doji with long shadows |
| CDLLONGLINE | Long Line Candle | `open, high, low, close` | Candle with very long body |
| CDLMARUBOZU | Marubozu | `open, high, low, close` | Candle with no shadows |
| CDLMATCHINGLOW | Matching Low | `open, high, low, close` | Bullish reversal pattern |
| CDLMATHOLD | Mat Hold | `open, high, low, close, penetration` | Bullish continuation pattern |
| CDLMORNINGDOJISTAR | Morning Doji Star | `open, high, low, close, penetration` | Bullish three-candle pattern |
| CDLMORNINGSTAR | Morning Star | `open, high, low, close, penetration` | Bullish reversal pattern |
| CDLONNECK | On-Neck Pattern | `open, high, low, close` | Continuation pattern |
| CDLPIERCING | Piercing Pattern | `open, high, low, close` | Bullish two-candle pattern |
| CDLRICKSHAWMAN | Rickshaw Man | `open, high, low, close, doji_pct` | Doji with equal shadows |
| CDLRISEFALL3METHODS | Rising/Falling Three Methods | `open, high, low, close` | Five-candle continuation |
| CDLSEPARATINGLINES | Separating Lines | `open, high, low, close` | Continuation pattern |
| CDLSHOOTINGSTAR | Shooting Star | `open, high, low, close` | Bearish reversal with upper shadow |
| CDLSHORTLINE | Short Line Candle | `open, high, low, close` | Candle with small body |
| CDLSPINNINGTOP | Spinning Top | `open, high, low, close` | Small body with long shadows |
| CDLSTALLEDPATTERN | Stalled Pattern | `open, high, low, close` | Bearish stall in uptrend |
| CDLSTICKSANDWICH | Stick Sandwich | `open, high, low, close` | Bullish three-candle pattern |
| CDLTAKURI | Takuri | `open, high, low, close` | Dragonfly with very long shadow |
| CDLTASUKIGAP | Tasuki Gap | `open, high, low, close` | Continuation with gap |
| CDLTHRUSTING | Thrusting Pattern | `open, high, low, close` | Bearish continuation |
| CDLTRISTAR | Tristar Pattern | `open, high, low, close, doji_pct` | Three doji pattern |
| CDLUNIQUE3RIVER | Unique 3 River | `open, high, low, close` | Bullish reversal pattern |
| CDLUPSIDEGAP2CROWS | Upside Gap Two Crows | `open, high, low, close` | Bearish three-candle pattern |
| CDLXSIDEGAP3METHODS | Side-by-Side White Lines | `open, high, low, close` | Continuation pattern |

### Chart Patterns (15+)

Chart patterns return arrays of detected pattern locations.

| Pattern | Parameters | Output | Description |
|---------|------------|--------|-------------|
| Head & Shoulders Top | `high, lookback, tolerance` | Array | Bearish reversal pattern |
| Head & Shoulders Bottom | `low, lookback, tolerance` | Array | Bullish inverse pattern |
| Double Top | `high, lookback, tolerance` | Array | Bearish reversal at resistance |
| Double Bottom | `low, lookback, tolerance` | Array | Bullish reversal at support |
| Triple Top | `high, lookback, tolerance` | Array | Stronger bearish reversal |
| Triple Bottom | `low, lookback, tolerance` | Array | Stronger bullish reversal |
| Ascending Triangle | `high, low, lookback, tolerance` | Array | Bullish continuation pattern |
| Descending Triangle | `high, low, lookback, tolerance` | Array | Bearish continuation pattern |
| Symmetrical Triangle | `high, low, lookback, tolerance` | Array | Breakout pattern |
| Rising Wedge | `high, low, lookback, tolerance` | Array | Bearish reversal pattern |
| Falling Wedge | `high, low, lookback, tolerance` | Array | Bullish reversal pattern |
| Pennant | `high, low, lookback, tolerance` | Array | Continuation pattern |
| Flag | `high, low, lookback, tolerance` | Array | Continuation after strong move |
| Rectangle | `high, low, lookback, tolerance` | Array | Consolidation pattern |
| Rounding Top | `high, lookback, tolerance` | Array | Gradual bearish reversal |
| Rounding Bottom | `low, lookback, tolerance` | Array | Gradual bullish reversal |

## Parameter Details

### Common Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `close` | Array\<f64\> | Required | Closing prices |
| `open` | Array\<f64\> | Required for some | Opening prices |
| `high` | Array\<f64\> | Required for some | Highest prices |
| `low` | Array\<f64\> | Required for some | Lowest prices |
| `volume` | Array\<f64\> | Required for volume indicators | Trading volume |
| `timeperiod` | u32 | 14 | Number of periods for calculation |
| `fastperiod` | u32 | 12 | Fast period for MACD-like indicators |
| `slowperiod` | u32 | 26 | Slow period for MACD-like indicators |
| `signalperiod` | u32 | 9 | Signal period for MACD |
| `nbdevup` | f64 | 2.0 | Upper standard deviation multiplier |
| `nbdevdn` | f64 | 2.0 | Lower standard deviation multiplier |
| `penetration` | f64 | 0.3 | Penetration threshold for patterns |
| `doji_pct` | f64 | 0.1 | Doji threshold percentage |
| `lookback` | u32 | 20 | Lookback period for chart patterns |
| `tolerance` | f64 | 0.03 | Tolerance for pattern matching |

### Moving Average Types (matype)

| Value | Type | Description |
|-------|------|-------------|
| 0 | SMA | Simple Moving Average |
| 1 | EMA | Exponential Moving Average |
| 2 | WMA | Weighted Moving Average |
| 3 | DEMA | Double Exponential Moving Average |
| 4 | TEMA | Triple Exponential Moving Average |
| 5 | TRIMA | Triangular Moving Average |
| 6 | KAMA | Kaufman Adaptive Moving Average |
| 7 | MAMA | MESA Adaptive Moving Average |
| 8 | T3 | T3 Moving Average |

## Usage Notes

1. All indicators return arrays of the same length as input data, with NaN values for periods where calculation is not possible
2. Candlestick patterns return 100 for bullish, -100 for bearish, 0 for no pattern
3. Chart patterns return indices where patterns are detected
4. Always validate input data for NaN and infinite values before calculation
5. For best performance, pre-allocate output arrays when possible

## Error Handling

All indicators return `Result<Array<TaError>` with the following error types:

| Error | Description |
|-------|-------------|
| `InvalidPeriod` | Period parameter is too small or negative |
| `InsufficientData` | Input array is too short for the specified period |
| `InvalidParameters` | Parameter values are out of valid range |
| `InvalidInput` | Input data contains NaN or infinite values |
| `ComputationError` | Internal calculation error occurred |
