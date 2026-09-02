package com.finkit;

/**
 * High-performance technical indicators powered by Rust via JNI.
 *
 * <p>All indicator functions operate on double arrays and return results as double arrays
 * of the same length. Initial values may be {@code Double.NaN} during the warm-up period
 * required by each indicator.
 *
 * <p>The native library ({@code libfinkit_java}) is automatically loaded on class
 * initialization. Supported platforms:
 * <ul>
 *   <li>Windows (x86_64) - {@code .dll}</li>
 *   <li>Linux (x86_64, aarch64) - {@code .so}</li>
 *   <li>macOS (x86_64, aarch64) - {@code .dylib}</li>
 * </ul>
 *
 * <p>Example usage:
 * <pre>{@code
 * double[] prices = {100.0, 101.0, 102.0, 103.0, 104.0, 105.0};
 * double[] sma = Indicators.sma(prices, 3);
 * }</pre>
 *
 * @since 0.1.0
 */
public final class Indicators {

    private Indicators() {
    }

    // =========================================================================
    // Native library loading
    // =========================================================================

    static {
        loadNativeLibrary();
    }

    private static void loadNativeLibrary() {
        String libName = "finkit_java";
        String os = System.getProperty("os.name", "").toLowerCase();
        String arch = System.getProperty("os.arch", "").toLowerCase();

        if (os.contains("win")) {
            System.loadLibrary(libName);
        } else if (os.contains("mac")) {
            System.loadLibrary(libName);
        } else if (os.contains("nix") || os.contains("nux") || os.contains("aix")) {
            System.loadLibrary(libName);
        } else {
            System.loadLibrary(libName);
        }
    }

    static void ensureLoaded() {
        // Calling this method triggers class initialization and native loading.
    }

    // =========================================================================
    // Overlap Studies
    // =========================================================================

    /**
     * Simple Moving Average (SMA).
     *
     * <p>Computes the arithmetic mean of the input over a rolling window.
     * The first {@code period-1} values will be {@code NaN}.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return SMA values
     */
    public static native double[] sma(double[] input, int period);

    /**
     * Exponential Moving Average (EMA).
     *
     * <p>EMA gives more weight to recent prices, reacting faster to price changes
     * than SMA. The first {@code period-1} values will be {@code NaN}.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return EMA values
     */
    public static native double[] ema(double[] input, int period);

    /**
     * Weighted Moving Average (WMA).
     *
     * <p>WMA assigns a linearly decreasing weight to each data point in the window,
     * with the most recent point having the highest weight.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return WMA values
     */
    public static native double[] wma(double[] input, int period);

    /**
     * Double Exponential Moving Average (DEMA).
     *
     * <p>DEMA reduces lag by using a combination of two EMAs. It reacts faster
     * to price changes than a single EMA.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return DEMA values
     */
    public static native double[] dema(double[] input, int period);

    /**
     * Triple Exponential Moving Average (TEMA).
     *
     * <p>TEMA further reduces lag compared to DEMA by using three EMAs.
     * Useful for short-term trading signals.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return TEMA values
     */
    public static native double[] tema(double[] input, int period);

    /**
     * Kaufman's Adaptive Moving Average (KAMA).
     *
     * <p>KAMA adapts to market volatility by adjusting its smoothing constant
     * based on the Efficiency Ratio (ER). In trending markets, KAMA behaves
     * like a fast MA; in choppy markets, it behaves like a slow MA.
     *
     * @param input  input data series
     * @param period number of periods for the Efficiency Ratio (must be &gt; 0)
     * @return KAMA values
     */
    public static native double[] kama(double[] input, int period);

    /**
     * Triple Exponential Moving Average with variable smoothing (T3).
     *
     * <p>T3 is a smooth moving average with reduced lag, using a volume factor
     * to control the smoothness. A vfactor of 0.7 is the default and provides
     * a good balance between smoothness and responsiveness.
     *
     * @param input   input data series
     * @param period  number of periods (must be &gt; 0)
     * @param vfactor volume factor, typically 0.7 (0 &lt; vfactor &lt; 1)
     * @return T3 values
     */
    public static native double[] t3(double[] input, int period, double vfactor);

    /**
     * MESA Adaptive Moving Average (MAMA).
     *
     * <p>MAMA uses the Hilbert Transform to adapt to market cycles. It produces
     * two lines: MAMA (the adaptive MA) and FAMA (Following Adaptive MA, a slower
     * version of MAMA).
     *
     * @param input     input data series
     * @param fastLimit fast limit (typically 0.5)
     * @param slowLimit slow limit (typically 0.05)
     * @result result object containing mama and fama arrays
     */
    public static native void mama(double[] input, double fastLimit, double slowLimit, MamaResult result);

    /**
     * Bollinger Bands (BBANDS).
     *
     * <p>Bollinger Bands consist of three lines: a middle band (SMA), an upper band
     * (middle + nbDevUp * standard deviation), and a lower band (middle - nbDevDn *
     * standard deviation). Commonly used to identify overbought/oversold conditions.
     *
     * @param input       input data series
     * @param timePeriod  number of periods for the SMA (must be &gt; 0)
     * @param nbDevUp     number of standard deviations for the upper band (typically 2.0)
     * @param nbDevDn     number of standard deviations for the lower band (typically 2.0)
     * @result result object containing upper, middle, and lower arrays
     */
    public static native void bbands(double[] input, int timePeriod, double nbDevUp, double nbDevDn, BbandsResult result);

    /**
     * MidPoint over period.
     *
     * <p>Returns the average of the highest and lowest values over the period.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return midpoint values
     */
    public static native double[] midpoint(double[] input, int period);

    /**
     * MidPrice over period.
     *
     * <p>Returns the average of the highest high and lowest low over the period.
     *
     * @param high   high price series
     * @param low    low price series
     * @param period number of periods (must be &gt; 0)
     * @return midprice values
     */
    public static native double[] midprice(double[] high, double[] low, int period);

    /**
     * Parabolic SAR (Stop and Reverse).
     *
     * <p>SAR provides potential reversal points in the price movement of an asset.
     * When price is above SAR, it suggests an uptrend; when below, a downtrend.
     *
     * @param high        high price series
     * @param low         low price series
     * @param close       close price series
     * @param acceleration acceleration factor (typically 0.02)
     * @param maximum     maximum acceleration factor (typically 0.2)
     * @result result object containing sar and af (acceleration factor) arrays
     */
    public static native void sar(double[] high, double[] low, double[] close, double acceleration, double maximum, SarResult result);

    // =========================================================================
    // Momentum Indicators
    // =========================================================================

    /**
     * Relative Strength Index (RSI).
     *
     * <p>RSI measures the magnitude of recent price changes to evaluate overbought
     * or oversold conditions. Values range from 0 to 100. Above 70 is typically
     * considered overbought; below 30 is oversold.
     *
     * @param input  input data series (typically close prices)
     * @param period number of periods (must be &gt; 0)
     * @return RSI values
     */
    public static native double[] rsi(double[] input, int period);

    /**
     * Moving Average Convergence Divergence (MACD).
     *
     * <p>MACD is a trend-following momentum indicator that shows the relationship
     * between two EMAs. It consists of three components: the MACD line, the signal
     * line (EMA of MACD), and the histogram (MACD - Signal).
     *
     * @param input        input data series
     * @param fastPeriod   fast EMA period (typically 12)
     * @param slowPeriod   slow EMA period (typically 26)
     * @param signalPeriod signal EMA period (typically 9)
     * @result result object containing macd, signal, and hist arrays
     */
    public static native void macd(double[] input, int fastPeriod, int slowPeriod, int signalPeriod, MacdResult result);

    /**
     * Stochastic Oscillator (STOCH).
     *
     * <p>The Stochastic Oscillator compares a closing price to its price range over
     * a given period. It consists of %K (fast) and %D (slow, SMA of %K).
     *
     * @param high    high price series
     * @param low     low price series
     * @param close   close price series
     * @param fastK   fast %K period (typically 14)
     * @param slowK   slow %K period (typically 3)
     * @param slowD   slow %D period (typically 3)
     * @result result object containing k and d arrays
     */
    public static native void stoch(double[] high, double[] low, double[] close, int fastK, int slowK, int slowD, StochResult result);

    /**
     * Average Directional Index (ADX).
     *
     * <p>ADX measures the strength of a trend regardless of direction. Values above
     * 25 indicate a strong trend; below 20 indicate a weak trend.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return ADX values
     */
    public static native double[] adx(double[] high, double[] low, double[] close, int period);

    /**
     * Aroon Indicator.
     *
     * <p>Aroon measures the time since the highest high and lowest low over a period.
     * Aroon Up measures time since the highest high; Aroon Down measures time since
     * the lowest low. Values range from 0 to 100.
     *
     * @param high   high price series
     * @param low    low price series
     * @param period number of periods (must be &gt; 0)
     * @result result object containing aroonUp and aroonDown arrays
     */
    public static native void aroon(double[] high, double[] low, int period, AroonResult result);

    /**
     * Commodity Channel Index (CCI).
     *
     * <p>CCI measures the deviation of the typical price from its statistical mean.
     * Values above +100 indicate overbought; below -100 indicate oversold.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return CCI values
     */
    public static native double[] cci(double[] high, double[] low, double[] close, int period);

    /**
     * Chande Momentum Oscillator (CMO).
     *
     * <p>CMO measures the momentum of price changes. Values range from -100 to +100.
     * Above +50 is bullish; below -50 is bearish.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return CMO values
     */
    public static native double[] cmo(double[] input, int period);

    /**
     * Directional Movement Index (DX).
     *
     * <p>DX measures the strength of a trend. It is derived from the +DI and -DI
     * indicators.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return DX values
     */
    public static native double[] dx(double[] high, double[] low, double[] close, int period);

    /**
     * Momentum (MOM).
     *
     * <p>Momentum measures the rate of change of price over a specified period.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return momentum values
     */
    public static native double[] mom(double[] input, int period);

    /**
     * Rate of Change (ROC).
     *
     * <p>ROC measures the percentage change in price over a specified period.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return ROC values
     */
    public static native double[] roc(double[] input, int period);

    /**
     * Williams %R (WILLR).
     *
     * <p>Williams %R is a momentum indicator that measures overbought/oversold levels.
     * Values range from -100 to 0. Above -20 is overbought; below -80 is oversold.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return Williams %R values
     */
    public static native double[] willr(double[] high, double[] low, double[] close, int period);

    /**
     * Absolute Price Oscillator (APO).
     *
     * <p>APO is the difference between two moving averages of different periods.
     *
     * @param input      input data series
     * @param fastPeriod fast period
     * @param slowPeriod slow period
     * @return APO values
     */
    public static native double[] apo(double[] input, int fastPeriod, int slowPeriod);

    /**
     * Balance of Power (BOP).
     *
     * <p>BOP measures the strength of buyers vs sellers by assessing the ability
     * to move price to an extreme level.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return BOP values
     */
    public static native double[] bop(double[] open, double[] high, double[] low, double[] close);

    /**
     * Money Flow Index (MFI).
     *
     * <p>MFI is a volume-weighted RSI that measures buying and selling pressure.
     * Values range from 0 to 100. Above 80 is overbought; below 20 is oversold.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param volume volume series
     * @param period number of periods (must be &gt; 0)
     * @return MFI values
     */
    public static native double[] mfi(double[] high, double[] low, double[] close, double[] volume, int period);

    /**
     * Plus Directional Indicator (+DI).
     *
     * <p>+DI measures the strength of upward price movement.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return +DI values
     */
    public static native double[] plusDi(double[] high, double[] low, double[] close, int period);

    /**
     * Minus Directional Indicator (-DI).
     *
     * <p>-DI measures the strength of downward price movement.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return -DI values
     */
    public static native double[] minusDi(double[] high, double[] low, double[] close, int period);

    /**
     * Triple Exponential Average (TRIX).
     *
     * <p>TRIX is a momentum oscillator that calculates the rate of change of a
     * triple-smoothed EMA. It filters out insignificant price movements.
     *
     * @param input  input data series
     * @param period number of periods (must be &gt; 0)
     * @return TRIX values
     */
    public static native double[] trix(double[] input, int period);

    // =========================================================================
    // Volume Indicators
    // =========================================================================

    /**
     * On Balance Volume (OBV).
     *
     * <p>OBV is a cumulative volume indicator that adds volume on up days and
     * subtracts volume on down days. It confirms price trends and spots divergences.
     *
     * @param close  close price series
     * @param volume volume series
     * @return OBV values
     */
    public static native double[] obv(double[] close, double[] volume);

    /**
     * Accumulation/Distribution Line (AD).
     *
     * <p>AD uses price and volume to assess whether an asset is being accumulated
     * or distributed. It considers the closing price relative to the high-low range.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param volume volume series
     * @return AD values
     */
    public static native double[] ad(double[] high, double[] low, double[] close, double[] volume);

    /**
     * AD Oscillator (ADOSC).
     *
     * <p>ADOSC is the difference between two EMAs (fast and slow) of the
     * Accumulation/Distribution Line. It shows short-term changes in the AD line.
     *
     * @param high       high price series
     * @param low        low price series
     * @param close      close price series
     * @param volume     volume series
     * @param fastPeriod fast EMA period (typically 3)
     * @param slowPeriod slow EMA period (typically 10)
     * @return ADOSC values
     */
    public static native double[] adosc(double[] high, double[] low, double[] close, double[] volume, int fastPeriod, int slowPeriod);

    // =========================================================================
    // Volatility Indicators
    // =========================================================================

    /**
     * Average True Range (ATR).
     *
     * <p>ATR measures market volatility by calculating the average of true ranges
     * over a period. Higher ATR indicates higher volatility.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return ATR values
     */
    public static native double[] atr(double[] high, double[] low, double[] close, int period);

    /**
     * Normalized Average True Range (NATR).
     *
     * <p>NATR is the ATR normalized by the closing price, expressed as a percentage.
     * Useful for comparing volatility across different price levels.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param period number of periods (must be &gt; 0)
     * @return NATR values (as percentage)
     */
    public static native double[] natr(double[] high, double[] low, double[] close, int period);

    /**
     * True Range (TRANGE).
     *
     * <p>True Range is the greatest of: high-low, |high-prevClose|, |low-prevClose|.
     *
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @return True Range values
     */
    public static native double[] trange(double[] high, double[] low, double[] close);

    // =========================================================================
    // Price Transforms
    // =========================================================================

    /**
     * Average Price.
     *
     * <p>Computes the average of open, high, low, and close: (O+H+L+C)/4.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return average price values
     */
    public static native double[] avgprice(double[] open, double[] high, double[] low, double[] close);

    /**
     * Median Price.
     *
     * <p>Computes the median of high and low: (H+L)/2.
     *
     * @param high high price series
     * @param low  low price series
     * @return median price values
     */
    public static native double[] medprice(double[] high, double[] low);

    /**
     * Typical Price.
     *
     * <p>Computes the typical price: (H+L+C)/3.
     *
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return typical price values
     */
    public static native double[] typprice(double[] high, double[] low, double[] close);

    /**
     * Weighted Close Price.
     *
     * <p>Computes the weighted close price: (H+L+2*C)/4.
     *
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return weighted close price values
     */
    public static native double[] wclprice(double[] high, double[] low, double[] close);

    // =========================================================================
    // Cycle Indicators (Hilbert Transform)
    // =========================================================================

    /**
     * Hilbert Transform - Dominant Cycle Period (HT_DCPERIOD).
     *
     * <p>Measures the dominant cycle period of the price series using the Hilbert
     * Transform. The dominant cycle period represents the most significant cycle
     * length in the data. Typically used with typical price as input.
     *
     * <p>Minimum 32 bars of data required for valid output.
     *
     * @param input input data series (typically typical price)
     * @return dominant cycle period values
     */
    public static native double[] htDcperiod(double[] input);

    /**
     * Hilbert Transform - Dominant Cycle Phase (HT_DCPHASE).
     *
     * <p>Measures the dominant cycle phase of the price series in degrees (0-360).
     * The phase indicates where the current price is within the dominant cycle.
     *
     * <p>Minimum 32 bars of data required for valid output.
     *
     * @param input input data series
     * @return dominant cycle phase values in degrees
     */
    public static native double[] htDcphase(double[] input);

    /**
     * Hilbert Transform - Phasor Components (HT_PHASOR).
     *
     * <p>Returns the in-phase and quadrature components of the Hilbert Transform.
     * These components represent the signal decomposed into two orthogonal parts.
     *
     * <p>Minimum 12 bars of data required for valid output.
     *
     * @param input input data series
     * @result result object containing inPhase and quadrature arrays
     */
    public static native void htPhasor(double[] input, HtPhasorResult result);

    /**
     * Hilbert Transform - Sine Wave (HT_SINE).
     *
     * <p>Returns the sine and lead sine wave components derived from the Hilbert
     * Transform. The lead sine is phase-shifted by 45 degrees. When the sine
     * crosses above the lead sine, it indicates the start of a new cycle.
     *
     * <p>Minimum 32 bars of data required for valid output.
     *
     * @param input input data series
     * @result result object containing sine and leadSine arrays
     */
    public static native void htSine(double[] input, HtSineResult result);

    /**
     * Hilbert Transform - Trend vs Cycle Mode (HT_TRENDMODE).
     *
     * <p>Indicates whether the market is in trend mode (1.0) or cycle mode (0.0).
     * Helps traders identify when to use trend-following vs cycle-based strategies.
     *
     * <p>Minimum 32 bars of data required for valid output.
     *
     * @param input input data series
     * @return mode values (1.0 for trend, 0.0 for cycle)
     */
    public static native double[] htTrendmode(double[] input);

    /**
     * Hilbert Transform - Instantaneous Trendline (HT_TRENDLINE).
     *
     * <p>Computes the instantaneous trendline of the price series using the Hilbert
     * Transform. The trendline represents the underlying trend with cycle components
     * removed.
     *
     * <p>Minimum 32 bars of data required for valid output.
     *
     * @param input input data series (typically typical price)
     * @return trendline values
     */
    public static native double[] htTrendline(double[] input);

    // =========================================================================
    // Statistical Indicators
    // =========================================================================

    /**
     * Z-Score.
     *
     * <p>Z-Score measures how many standard deviations a data point is from the
     * rolling mean. Useful for identifying extreme deviations from the mean.
     *
     * @param input  input data series
     * @param period number of periods for rolling calculation (must be &gt; 0)
     * @return Z-Score values
     */
    public static native double[] zscore(double[] input, int period);

    /**
     * Percent Rank.
     *
     * <p>Percent Rank shows the percentage of values in the lookback period that
     * are below the current value. Values range from 0 to 100.
     *
     * @param input  input data series
     * @param period lookback period (must be &gt; 0)
     * @return percent rank values
     */
    public static native double[] percentRank(double[] input, int period);

    /**
     * Beta.
     *
     * <p>Beta measures the systematic risk of an asset relative to a benchmark.
     * Beta &gt; 1 means the asset is more volatile than the benchmark.
     *
     * @param asset     asset return series
     * @param benchmark benchmark return series
     * @param period    number of periods for rolling calculation (must be &gt; 0)
     * @return beta values
     */
    public static native double[] beta(double[] asset, double[] benchmark, int period);

    /**
     * Pearson Correlation Coefficient.
     *
     * <p>Measures the linear correlation between two data series over a rolling
     * window. Values range from -1 (perfect negative) to +1 (perfect positive).
     *
     * @param inputA first data series
     * @param inputB second data series
     * @param period lookback period (must be &gt; 0)
     * @return correlation values
     */
    public static native double[] correlation(double[] inputA, double[] inputB, int period);

    /**
     * Standard Deviation.
     *
     * <p>Computes the rolling standard deviation of the input series.
     *
     * @param input  input data series
     * @param period lookback period (must be &gt; 0)
     * @param nbDev  number of standard deviations (typically 1.0)
     * @return standard deviation values
     */
    public static native double[] stdDev(double[] input, int period, double nbDev);

    /**
     * Linear Regression.
     *
     * <p>Fits a linear regression line to the data over a rolling window and
     * returns the fitted values.
     *
     * @param input  input data series
     * @param period lookback period (must be &gt; 0)
     * @return linear regression fitted values
     */
    public static native double[] linearReg(double[] input, int period);

    /**
     * Time Series Forecast (TSF).
     *
     * <p>TSF extends the linear regression by forecasting one period ahead.
     * It represents where the regression line would project to the next period.
     *
     * @param input  input data series
     * @param period lookback period (must be &gt; 0)
     * @return TSF values
     */
    public static native double[] tsf(double[] input, int period);

    // =========================================================================
    // ========================================================================
    // Advanced indicators and chart transforms
    // ========================================================================

    public static native void ichimoku(double[] high, double[] low, double[] close,
                                                   int tenkanPeriod, int kijunPeriod,
                                                   int senkouBPeriod, int displacement,
                                                   IchimokuResult result);

    public static native void supertrend(double[] high, double[] low, double[] close,
                                                      int atrPeriod, double multiplier,
                                                      SupertrendResult result);

    public static native double[] vwap(double[] high, double[] low, double[] close, double[] volume);
    public static native double[] anchoredVwap(double[] high, double[] low, double[] close,
                                                double[] volume, int startIndex);
    public static native void vwapBands(double[] high, double[] low, double[] close, double[] volume,
                                        int timePeriod, double nbDev, VwapBandsResult result);
    public static native void elderRay(double[] high, double[] low, double[] close, double[] volume,
                                       int period, ElderRayResult result);
    public static native void donchian(double[] high, double[] low, int period,
                                       DonchianResult result);
    public static native void volumeProfile(double[] high, double[] low, double[] close,
                                            double[] volume, int numBins,
                                            VolumeProfileResult result);
    public static native void fibonacciRetracement(double[] high, double[] low,
                                                    int startIndex, int endIndex,
                                                    FibonacciRetracementResult result);

    public static native DoubleDoubleIntOutput darvasBox(double[] high, double[] low, double[] close,
                                                          int lookback, int confirmation);
    public static native DoubleIntIntOutput pointAndFigure(double[] high, double[] low,
                                                            double boxSize, int reversal);
    public static native DoubleIntOutput threeLineBreak(double[] close, int lines);
    public static native TripleDoubleOutput williamsAlligator(double[] close);
    public static native QuadOutput heikinAshi(double[] open, double[] high, double[] low,
                                                double[] close);
    public static native DoubleIntOutput renko(double[] high, double[] low, double boxSize);
    public static native DoubleIntOutput kagi(double[] close, double reversal);

    // ========================================================================
    // Advanced indicators, chart transforms, and formula JSON helpers
    // ========================================================================

    public static native void ichimoku(double[] high, double[] low, double[] close,
                                        int tenkanPeriod, int kijunPeriod,
                                        int senkouBPeriod, int displacement,
                                        IchimokuResult result);
    public static native void supertrend(double[] high, double[] low, double[] close,
                                         int atrPeriod, double multiplier,
                                         SupertrendResult result);
    public static native double[] vwap(double[] high, double[] low, double[] close, double[] volume);
    public static native double[] anchoredVwap(double[] high, double[] low, double[] close,
                                               double[] volume, int startIndex);
    public static native void vwapBands(double[] high, double[] low, double[] close, double[] volume,
                                        int timePeriod, double nbDev, VwapBandsResult result);
    public static native void elderRay(double[] high, double[] low, double[] close, double[] volume,
                                       int period, ElderRayResult result);
    public static native void donchian(double[] high, double[] low, int period,
                                       DonchianResult result);
    public static native void volumeProfile(double[] high, double[] low, double[] close,
                                            double[] volume, int numBins,
                                            VolumeProfileResult result);
    public static native void fibonacciRetracement(double[] high, double[] low,
                                                    int startIndex, int endIndex,
                                                    FibonacciRetracementResult result);

    public static native DoubleDoubleIntOutput darvasBox(double[] high, double[] low, double[] close,
                                                          int lookback, int confirmation);
    public static native DoubleIntIntOutput pointAndFigure(double[] high, double[] low,
                                                            double boxSize, int reversal);
    public static native DoubleIntOutput threeLineBreak(double[] close, int lines);
    public static native TripleDoubleOutput williamsAlligator(double[] close);
    public static native QuadOutput heikinAshi(double[] open, double[] high, double[] low,
                                                double[] close);
    public static native DoubleIntOutput renko(double[] high, double[] low, double boxSize);
    public static native DoubleIntOutput kagi(double[] close, double reversal);

    public static native String formulaEvalMulti(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);
    public static native String formulaEvalDraw(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);
    public static native String formulaEvalDebug(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);
    public static native String formulaGetTemplate(String name);
    public static native String formulaSearchTemplates(String keyword);
    public static native String formulaListCategories();

    // Additional Result Classes
    // =========================================================================

    /**
     * Result container for Parabolic SAR indicator.
     */
    public static final class SarResult {
        /** SAR values */
        public double[] sar;
        /** Acceleration factor values */
        public double[] af;
    }

    /**
     * Result container for Aroon indicator.
     */
    public static final class AroonResult {
        /** Aroon Up values */
        public double[] aroonUp;
        /** Aroon Down values */
        public double[] aroonDown;
    }

    // =========================================================================
    // Formula Engine
    // =========================================================================

    /**
     * Evaluates a trading formula string against OHLCV data.
     *
     * <p>Compiles and executes a trading formula string similar to
     * TongDaXin (通达信) formula language.
     *
     * @param source formula source code
     * @param open   open price series
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param volume volume series
     * @return HashMap with variable names as keys and double arrays as values.
     *         The special key "__final__" contains the final expression result.
     */
    public static native java.util.HashMap<String, double[]> formulaEval(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);

    /**
     * Validates a formula source string for syntactic correctness.
     *
     * @param source formula source code to validate
     * @return true if the formula is valid, false otherwise
     */
    public static native boolean formulaValidate(String source);

    /**
     * Evaluates a trading formula with JIT compilation.
     *
     * <p>Compiles the formula using Just-In-Time compilation for maximum execution speed.
     * This is ideal for formulas that need to be executed repeatedly with different data.
     *
     * @param source formula source code
     * @param open   open price series
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param volume volume series
     * @return HashMap with variable names as keys and double arrays as values.
     *         The special key "__final__" contains the final expression result.
     */
    public static native java.util.HashMap<String, double[]> formulaEvalJit(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);

    /**
     * Evaluates a trading formula with SIMD optimization.
     *
     * <p>Uses SIMD (Single Instruction Multiple Data) vectorization to accelerate
     * formula execution on supported hardware. Best suited for data-parallel
     * operations on large datasets.
     *
     * @param source formula source code
     * @param open   open price series
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param volume volume series
     * @return HashMap with variable names as keys and double arrays as values.
     *         The special key "__final__" contains the final expression result.
     */
    public static native java.util.HashMap<String, double[]> formulaEvalSimd(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);

    /**
     * Evaluates a trading formula with zero-copy optimization.
     *
     * <p>Minimizes memory allocations by operating directly on input buffers
     * without copying data. This provides the lowest latency execution path
     * for latency-sensitive applications.
     *
     * @param source formula source code
     * @param open   open price series
     * @param high   high price series
     * @param low    low price series
     * @param close  close price series
     * @param volume volume series
     * @return HashMap with variable names as keys and double arrays as values.
     *         The special key "__final__" contains the final expression result.
     */
    public static native java.util.HashMap<String, double[]> formulaEvalZeroCopy(
        String source, double[] open, double[] high, double[] low, double[] close, double[] volume);
}
