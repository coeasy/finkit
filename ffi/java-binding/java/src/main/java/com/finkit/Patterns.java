package com.finkit;

/**
 * Candlestick pattern detection powered by Rust via JNI.
 *
 * <p>All pattern detection functions return an {@code int[]} array of the same length
 * as the input OHLC arrays. Each element indicates the pattern signal at that bar:
 * <ul>
 *   <li>{@code 100}  - Bullish pattern detected</li>
 *   <li>{@code -100} - Bearish pattern detected</li>
 *   <li>{@code 0}    - No pattern detected</li>
 * </ul>
 *
 * <p>Example usage:
 * <pre>{@code
 * int[] signals = Patterns.cdlHammer(open, high, low, close);
 * for (int i = 0; i < signals.length; i++) {
 *     if (signals[i] == 100) {
 *         System.out.println("Bullish hammer at bar " + i);
 *     }
 * }
 * }</pre>
 *
 * @since 0.1.2
 */
public final class Patterns {

    private Patterns() {
    }

    // =========================================================================
    // Single Candle Patterns
    // =========================================================================

    /**
     * Doji pattern.
     *
     * <p>A Doji forms when open and close are virtually equal, indicating market indecision.
     * Uses default threshold (body less than 10% of the total range).
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlDoji(double[] open, double[] high, double[] low, double[] close);

    /**
     * Doji pattern with custom threshold.
     *
     * @param open     open price series
     * @param high     high price series
     * @param low      low price series
     * @param close    close price series
     * @param dojiPct  maximum body-to-range ratio for Doji (e.g., 0.1 for 10%)
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlDojiWithThreshold(double[] open, double[] high, double[] low, double[] close, double dojiPct);

    /**
     * Dragonfly Doji pattern.
     *
     * <p>A Doji with a long lower shadow and little to no upper shadow, typically
     * found at the bottom of a downtrend. Signals potential bullish reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlDragonflyDoji(double[] open, double[] high, double[] low, double[] close);

    /**
     * Gravestone Doji pattern.
     *
     * <p>A Doji with a long upper shadow and little to no lower shadow, typically
     * found at the top of an uptrend. Signals potential bearish reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlGravestoneDoji(double[] open, double[] high, double[] low, double[] close);

    /**
     * Long-Legged Doji pattern.
     *
     * <p>A Doji with long upper and lower shadows, indicating high volatility
     * and indecision.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlLongLeggedDoji(double[] open, double[] high, double[] low, double[] close);

    /**
     * Four-Price Doji pattern.
     *
     * <p>All four prices (open, high, low, close) are equal. Rare pattern indicating
     * extreme illiquidity or a paused market.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlDoji4Prices(double[] open, double[] high, double[] low, double[] close);

    /**
     * Marubozu pattern.
     *
     * <p>A long candle with no (or very small) shadows. A white Marubozu is bullish;
     * a black Marubozu is bearish.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMarubozu(double[] open, double[] high, double[] low, double[] close);

    /**
     * Marubozu pattern with custom shadow threshold.
     *
     * @param open        open price series
     * @param high        high price series
     * @param low         low price series
     * @param close       close price series
     * @param shadowPct   maximum shadow-to-body ratio (e.g., 0.1)
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMarubozuWithThreshold(double[] open, double[] high, double[] low, double[] close, double shadowPct);

    /**
     * Hammer pattern.
     *
     * <p>A small body at the upper end with a long lower shadow (at least 2x body),
     * found in a downtrend. Signals potential bullish reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlHammer(double[] open, double[] high, double[] low, double[] close);

    /**
     * Inverted Hammer pattern.
     *
     * <p>A small body at the lower end with a long upper shadow, found in a downtrend.
     * Signals potential bullish reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlInvertedHammer(double[] open, double[] high, double[] low, double[] close);

    /**
     * Hanging Man pattern.
     *
     * <p>Same shape as a Hammer but found in an uptrend. Signals potential bearish reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlHangingMan(double[] open, double[] high, double[] low, double[] close);

    /**
     * Shooting Star pattern.
     *
     * <p>Same shape as an Inverted Hammer but found in an uptrend. Signals potential
     * bearish reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlShootingStar(double[] open, double[] high, double[] low, double[] close);

    /**
     * Spinning Top pattern.
     *
     * <p>A small body with upper and lower shadows of similar length, indicating
     * market indecision.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlSpinningTop(double[] open, double[] high, double[] low, double[] close);

    /**
     * High Wave pattern.
     *
     * <p>A candle with a very small body and very long upper and lower shadows,
     * indicating extreme indecision.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlHighWave(double[] open, double[] high, double[] low, double[] close);

    /**
     * Rickshaw Man pattern.
     *
     * <p>A High Wave with the open near the midpoint of the range. Indicates
     * strong market indecision.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlRickshawMan(double[] open, double[] high, double[] low, double[] close);

    /**
     * Short Line candle pattern.
     *
     * <p>A candle with a very small body (less than 10% of the 10-bar average range).
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlShortLine(double[] open, double[] high, double[] low, double[] close);

    /**
     * Long Line candle pattern.
     *
     * <p>A candle with a very large body (more than 2x the 10-bar average range).
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlLongLine(double[] open, double[] high, double[] low, double[] close);

    /**
     * Belt Hold pattern.
     *
     * <p>A long candle that opens at the high (black) or low (white) of the day
     * with no shadow on one side.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlBeltHold(double[] open, double[] high, double[] low, double[] close);

    /**
     * Closing Marubozu pattern.
     *
     * <p>A candle with no shadow at the close end (closes at high or low).
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlClosingMarubozu(double[] open, double[] high, double[] low, double[] close);

    // =========================================================================
    // Dual Candle Patterns
    // =========================================================================

    /**
     * Engulfing pattern.
     *
     * <p>The second candle's body completely engulfs the first candle's body.
     * Bullish engulfing occurs in a downtrend; bearish engulfing in an uptrend.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlEngulfing(double[] open, double[] high, double[] low, double[] close);

    /**
     * Harami pattern.
     *
     * <p>The second candle's body is contained within the first candle's body.
     * Indicates a potential trend reversal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlHarami(double[] open, double[] high, double[] low, double[] close);

    /**
     * Harami Cross pattern.
     *
     * <p>A Harami where the second candle is a Doji. Stronger reversal signal
     * than a regular Harami.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlHaramiCross(double[] open, double[] high, double[] low, double[] close);

    /**
     * Piercing pattern.
     *
     * <p>A two-candle bullish reversal pattern. The second candle opens below the
     * previous low and closes above the midpoint of the first candle.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlPiercing(double[] open, double[] high, double[] low, double[] close);

    /**
     * Dark Cloud Cover pattern.
     *
     * <p>A two-candle bearish reversal pattern. The second candle opens above the
     * previous high and closes below the midpoint of the first candle.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlDarkCloudCover(double[] open, double[] high, double[] low, double[] close);

    /**
     * Tweezer Top pattern.
     *
     * <p>Two candles with matching highs at a peak, indicating resistance.
     * Bearish reversal signal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlTweezerTop(double[] open, double[] high, double[] low, double[] close);

    /**
     * Tweezer Bottom pattern.
     *
     * <p>Two candles with matching lows at a trough, indicating support.
     * Bullish reversal signal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlTweezerBot(double[] open, double[] high, double[] low, double[] close);

    /**
     * Thrusting pattern.
     *
     * <p>A two-candle bearish continuation pattern. The second candle closes below
     * the midpoint of the first candle but above its low.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThrusting(double[] open, double[] high, double[] low, double[] close);

    /**
     * In-Neck pattern.
     *
     * <p>A two-candle bearish continuation pattern. The second candle closes near
     * the low of the first candle.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlInNeck(double[] open, double[] high, double[] low, double[] close);

    /**
     * On-Neck pattern.
     *
     * <p>A two-candle bearish continuation pattern. The second candle closes at or
     * very near the low of the first candle.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlOnNeck(double[] open, double[] high, double[] low, double[] close);

    // =========================================================================
    // Triple Candle Patterns
    // =========================================================================

    /**
     * Morning Star pattern.
     *
     * <p>A three-candle bullish reversal pattern: a long black candle, a small-bodied
     * candle that gaps down, and a long white candle that closes well into the first
     * candle's body.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMorningStar(double[] open, double[] high, double[] low, double[] close);

    /**
     * Evening Star pattern.
     *
     * <p>A three-candle bearish reversal pattern: a long white candle, a small-bodied
     * candle that gaps up, and a long black candle that closes well into the first
     * candle's body.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlEveningStar(double[] open, double[] high, double[] low, double[] close);

    /**
     * Morning Doji Star pattern.
     *
     * <p>A Morning Star where the middle candle is a Doji. Stronger bullish reversal signal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMorningDojiStar(double[] open, double[] high, double[] low, double[] close);

    /**
     * Morning Doji Star with custom Doji threshold.
     *
     * @param open    open price series
     * @param high    high price series
     * @param low     low price series
     * @param close   close price series
     * @param dojiPct maximum body-to-range ratio for Doji
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMorningDojiStarWithThreshold(double[] open, double[] high, double[] low, double[] close, double dojiPct);

    /**
     * Evening Doji Star pattern.
     *
     * <p>An Evening Star where the middle candle is a Doji. Stronger bearish reversal signal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlEveningDojiStar(double[] open, double[] high, double[] low, double[] close);

    /**
     * Evening Doji Star with custom Doji threshold.
     *
     * @param open    open price series
     * @param high    high price series
     * @param low     low price series
     * @param close   close price series
     * @param dojiPct maximum body-to-range ratio for Doji
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlEveningDojiStarWithThreshold(double[] open, double[] high, double[] low, double[] close, double dojiPct);

    /**
     * Three White Soldiers pattern.
     *
     * <p>Three consecutive long white (bullish) candles with higher closes, each
     * opening within the previous candle's body. Strong bullish reversal signal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeWhiteSoldiers(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three Black Crows pattern.
     *
     * <p>Three consecutive long black (bearish) candles with lower closes, each
     * opening within the previous candle's body. Strong bearish reversal signal.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeBlackCrows(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three Inside Up pattern.
     *
     * <p>A bullish reversal pattern: a large black candle, followed by a Harami
     * (small white candle within the first), and confirmed by a third white candle
     * closing above the first candle's high.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeInsideUp(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three Outside Up pattern.
     *
     * <p>A bullish reversal pattern: a bullish engulfing followed by a third white
     * candle closing higher.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeOutsideUp(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three Inside Down pattern.
     *
     * <p>A bearish reversal pattern: a large white candle, followed by a Harami
     * (small black candle within the first), and confirmed by a third black candle
     * closing below the first candle's low.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeInsideDown(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three Outside Down pattern.
     *
     * <p>A bearish reversal pattern: a bearish engulfing followed by a third black
     * candle closing lower.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeOutsideDown(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three Stars in the South pattern.
     *
     * <p>A rare three-candle bullish reversal pattern found at the bottom of a
     * downtrend, consisting of three small black candles with progressively lower
     * shadows.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeStarsInSouth(double[] open, double[] high, double[] low, double[] close);

    /**
     * Three-Line Strike pattern.
     *
     * <p>A four-candle pattern: three consecutive candles in the same direction
     * followed by a fourth candle that engulfs all three. Bullish version occurs
     * in a downtrend.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlThreeLineStrike(double[] open, double[] high, double[] low, double[] close);

    /**
     * Stick Sandwich pattern.
     *
     * <p>A bullish pattern: a black candle, then a white candle, then another black
     * candle at the same low as the first. Indicates support.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlStickSandwich(double[] open, double[] high, double[] low, double[] close);

    /**
     * Abandoned Baby pattern.
     *
     * <p>A rare three-candle reversal pattern with a Doji that gaps away from the
     * surrounding candles. Uses default penetration threshold (30%).
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlAbandonedBaby(double[] open, double[] high, double[] low, double[] close);

    /**
     * Abandoned Baby with custom penetration threshold.
     *
     * @param open           open price series
     * @param high           high price series
     * @param low            low price series
     * @param close          close price series
     * @param penetrationPct minimum penetration into the first candle's body (e.g., 0.3)
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlAbandonedBabyWithThreshold(double[] open, double[] high, double[] low, double[] close, double penetrationPct);

    // =========================================================================
    // Complex / Multi-Candle Patterns
    // =========================================================================

    /**
     * Upside Gap Two Crows pattern.
     *
     * <p>A bearish continuation pattern: a white candle, a gap up, then two black
     * candles that fill the gap.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlUpsideGap2Crows(double[] open, double[] high, double[] low, double[] close);

    /**
     * Upside/Downside Gap Three Methods pattern.
     *
     * <p>A continuation pattern involving gaps and three candles.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlUpsideGap3Methods(double[] open, double[] high, double[] low, double[] close);

    /**
     * Mat Hold pattern.
     *
     * <p>A bullish continuation pattern: a long white candle, a gap up, then three
     * small candles that stay above the midpoint of the first candle, followed by
     * another long white candle.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMatHold(double[] open, double[] high, double[] low, double[] close);

    /**
     * Tasuki Gap pattern.
     *
     * <p>A continuation pattern: two candles in the same direction with a gap,
     * followed by a candle in the opposite direction that opens within the gap
     * but does not close it.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlTasukiGap(double[] open, double[] high, double[] low, double[] close);

    /**
     * Separating Lines pattern.
     *
     * <p>A continuation pattern: two candles with the same open price but in
     * opposite directions, indicating the prior trend will continue.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlSeparatingLines(double[] open, double[] high, double[] low, double[] close);

    /**
     * Counter Attack pattern.
     *
     * <p>A reversal pattern: a long candle in the trend direction, followed by
     * a long candle in the opposite direction that closes at the same level.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlCounterAttack(double[] open, double[] high, double[] low, double[] close);

    /**
     * Matching Low pattern.
     *
     * <p>A bullish reversal pattern: two black candles with matching closes at
     * a low point.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlMatchingLow(double[] open, double[] high, double[] low, double[] close);

    /**
     * Identical Three Crows pattern.
     *
     * <p>A bearish reversal pattern: three consecutive black candles with matching
     * closes.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlIdentical3Crows(double[] open, double[] high, double[] low, double[] close);

    /**
     * Unique Three River pattern.
     *
     * <p>A bullish reversal pattern: three candles forming a unique river-like
     * shape at the bottom of a downtrend.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlUnique3River(double[] open, double[] high, double[] low, double[] close);

    /**
     * Breakaway pattern.
     *
     * <p>A five-candle reversal pattern: a long candle in the trend direction,
     * a gap, then three candles with progressively lower/higher closes, and a
     * final long candle in the opposite direction.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlBreakaway(double[] open, double[] high, double[] low, double[] close);

    /**
     * Concealing Baby Swallow pattern.
     *
     * <p>A rare four-candle bullish reversal pattern found in a downtrend,
     * involving two consecutive black Marubozu candles followed by a third
     * that gaps down and is engulfed by a fourth white candle.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlConcealingBabySwallow(double[] open, double[] high, double[] low, double[] close);

    /**
     * Kicking pattern.
     *
     * <p>A strong reversal pattern: two Marubozu candles in opposite directions
     * with a gap between them.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlKicking(double[] open, double[] high, double[] low, double[] close);

    /**
     * Kicking by Length pattern.
     *
     * <p>A Kicking pattern that also considers the relative lengths of the
     * two Marubozu candles.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlKickingByLength(double[] open, double[] high, double[] low, double[] close);

    /**
     * Advance Block pattern.
     *
     * <p>A bearish reversal pattern: three consecutive white candles with
     * progressively smaller bodies and/or longer upper shadows, indicating
     * weakening bullish momentum.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlAdvanceBlock(double[] open, double[] high, double[] low, double[] close);

    /**
     * Stalled Pattern.
     *
     * <p>A bearish reversal pattern: three white candles where the third candle
     * is small and closes near the high, indicating loss of momentum.
     *
     * @param open  open price series
     * @param high  high price series
     * @param low   low price series
     * @param close close price series
     * @return pattern signals (100/-100/0)
     */
    public static native int[] cdlStalledPattern(double[] open, double[] high, double[] low, double[] close);
}
