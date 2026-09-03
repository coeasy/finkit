package com.finkit;

/**
 * Chart pattern detection powered by Rust via JNI.
 *
 * <p>Chart patterns are multi-bar formations that indicate potential trend reversals
 * or continuations. All detection functions return an {@code int[]} array of the same
 * length as the input price arrays, with the index where the pattern completes marked.
 *
 * <p>Return values:
 * <ul>
 *   <li>{@code 1} - Bullish pattern detected at this index</li>
 *   <li>{@code -1} - Bearish pattern detected at this index</li>
 *   <li>{@code 0} - No pattern detected</li>
 * </ul>
 *
 * <p>Example usage:
 * <pre>{@code
 * int[] patterns = ChartPatterns.detectHeadShouldersTop(high, 10, 1.1);
 * for (int i = 0; i < patterns.length; i++) {
 *     if (patterns[i] == -1) {
 *         System.out.println("Head and Shoulders Top completed at bar " + i);
 *     }
 * }
 * }</pre>
 *
 * @since 0.1.2
 */
public final class ChartPatterns {

    private ChartPatterns() {
    }

    // =========================================================================
    // Reversal Patterns
    // =========================================================================

    /**
     * Head and Shoulders Top detection.
     *
     * <p>A bearish reversal pattern consisting of three peaks: a higher middle peak
     * (head) flanked by two lower peaks (shoulders). The pattern completes when price
     * breaks below the neckline (line connecting the two troughs).
     *
     * @param high      high price series
     * @param minBars   minimum bars between peaks (typically 10)
     * @param headRatio minimum head-to-shoulder height ratio (typically 1.1)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectHeadShouldersTop(double[] high, int minBars, double headRatio);

    /**
     * Head and Shoulders Bottom (inverse head-and-shoulders) detection.
     *
     * <p>A bullish reversal pattern consisting of three troughs: a lower middle trough
     * (head) flanked by two higher troughs (shoulders). The pattern completes when price
     * breaks above the neckline.
     *
     * @param low       low price series
     * @param minBars   minimum bars between troughs (typically 10)
     * @param headRatio minimum shoulder-to-head depth ratio (typically 1.1)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectHeadShouldersBottom(double[] low, int minBars, double headRatio);

    /**
     * Double Top detection.
     *
     * <p>A bearish reversal pattern where price tests a resistance level twice and
     * fails to break through, forming two peaks at approximately the same level.
     *
     * @param high      high price series
     * @param lookback  lookback period for pattern detection (typically 20-50)
     * @param tolerance tolerance for peak height matching as a ratio (e.g., 0.03 for 3%)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectDoubleTop(double[] high, int lookback, double tolerance);

    /**
     * Double Bottom detection.
     *
     * <p>A bullish reversal pattern where price tests a support level twice and
     * fails to break through, forming two troughs at approximately the same level.
     *
     * @param low       low price series
     * @param lookback  lookback period for pattern detection (typically 20-50)
     * @param tolerance tolerance for trough depth matching as a ratio (e.g., 0.03 for 3%)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectDoubleBottom(double[] low, int lookback, double tolerance);

    /**
     * Triple Top detection.
     *
     * <p>A bearish reversal pattern with three peaks at approximately the same level.
     * Stronger than a Double Top due to the additional failed breakout attempt.
     *
     * @param high      high price series
     * @param lookback  lookback period for pattern detection (typically 30-60)
     * @param tolerance tolerance for peak height matching as a ratio (e.g., 0.03 for 3%)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectTripleTop(double[] high, int lookback, double tolerance);

    /**
     * Triple Bottom detection.
     *
     * <p>A bullish reversal pattern with three troughs at approximately the same level.
     * Stronger than a Double Bottom due to the additional failed breakdown attempt.
     *
     * @param low       low price series
     * @param lookback  lookback period for pattern detection (typically 30-60)
     * @param tolerance tolerance for trough depth matching as a ratio (e.g., 0.03 for 3%)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectTripleBottom(double[] low, int lookback, double tolerance);

    // =========================================================================
    // Triangle Patterns
    // =========================================================================

    /**
     * Ascending Triangle detection.
     *
     * <p>A bullish continuation pattern with a flat resistance line (upper trendline)
     * and a rising support line (lower trendline). The converging trendlines indicate
     * building buying pressure.
     *
     * @param high      high price series
     * @param low       low price series
     * @param lookback  lookback period for pattern detection (typically 20-50)
     * @param tolerance tolerance for trendline flatness as a ratio
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectAscendingTriangle(double[] high, double[] low, int lookback, double tolerance);

    /**
     * Descending Triangle detection.
     *
     * <p>A bearish continuation pattern with a flat support line (lower trendline)
     * and a falling resistance line (upper trendline). The converging trendlines
     * indicate building selling pressure.
     *
     * @param high      high price series
     * @param low       low price series
     * @param lookback  lookback period for pattern detection (typically 20-50)
     * @param tolerance tolerance for trendline flatness as a ratio
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectDescendingTriangle(double[] high, double[] low, int lookback, double tolerance);

    /**
     * Symmetrical Triangle detection.
     *
     * <p>A continuation pattern with two converging trendlines sloping toward each other.
     * The direction of breakout is uncertain, but the prior trend often continues.
     *
     * @param high     high price series
     * @param low      low price series
     * @param lookback lookback period for pattern detection (typically 20-50)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectSymmetricalTriangle(double[] high, double[] low, int lookback);

    // =========================================================================
    // Wedge Patterns
    // =========================================================================

    /**
     * Rising Wedge detection.
     *
     * <p>A bearish reversal (or bearish continuation) pattern with two rising
     * converging trendlines. The narrowing range indicates weakening momentum
     * and often leads to a downward breakout.
     *
     * @param high     high price series
     * @param low      low price series
     * @param lookback lookback period for pattern detection (typically 15-40)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectRisingWedge(double[] high, double[] low, int lookback);

    /**
     * Falling Wedge detection.
     *
     * <p>A bullish reversal (or bullish continuation) pattern with two falling
     * converging trendlines. The narrowing range indicates weakening downward
     * momentum and often leads to an upward breakout.
     *
     * @param high     high price series
     * @param low      low price series
     * @param lookback lookback period for pattern detection (typically 15-40)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectFallingWedge(double[] high, double[] low, int lookback);

    // =========================================================================
    // Continuation Patterns
    // =========================================================================

    /**
     * Pennant detection.
     *
     * <p>A continuation pattern consisting of a strong price move (flagpole) followed
     * by a small symmetrical triangle (pennant). The pattern signals a brief pause
     * before the trend resumes.
     *
     * @param high            high price series
     * @param low             low price series
     * @param close           close price series
     * @param flagpolePeriod  expected flagpole length in bars (typically 5-10)
     * @param pennantPeriod   expected pennant length in bars (typically 5-20)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectPennant(double[] high, double[] low, double[] close,
                                             int flagpolePeriod, int pennantPeriod);

    /**
     * Flag detection.
     *
     * <p>A continuation pattern consisting of a strong price move (flagpole) followed
     * by a small rectangular channel sloping against the trend (flag). The pattern
     * signals a brief consolidation before the trend resumes.
     *
     * @param high           high price series
     * @param low            low price series
     * @param close          close price series
     * @param flagpolePeriod expected flagpole length in bars (typically 5-10)
     * @param flagPeriod     expected flag length in bars (typically 5-20)
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectFlag(double[] high, double[] low, double[] close,
                                          int flagpolePeriod, int flagPeriod);

    /**
     * Rectangle (Trading Range) detection.
     *
     * <p>A consolidation pattern where price oscillates between parallel support
     * and resistance levels. Can be a continuation or reversal pattern depending
     * on the breakout direction.
     *
     * @param high      high price series
     * @param low       low price series
     * @param lookback  lookback period for pattern detection (typically 20-60)
     * @param tolerance tolerance for boundary flatness as a ratio
     * @return pattern signals (1/-1/0)
     */
    public static native int[] detectRectangle(double[] high, double[] low, int lookback, double tolerance);
}
