package com.finkit.indicators;

/**
 * High-performance financial technical analysis for Android, backed by
 * the Finkit Rust core via JNI.
 *
 * <p>The class loads {@code finkit_android} automatically when it is first
 * referenced. The AAR must therefore contain the matching native library
 * under {@code jni/<abi>/} for the device ABI.
 *
 * <p>All public indicator methods delegate to pure Rust calculations. The
 * JNI boundary copies Java {@code double[]} inputs into Rust-owned buffers
 * for the duration of each call.
 */
public final class Finkit {

    static {
        try {
            System.loadLibrary("finkit_android");
        } catch (UnsatisfiedLinkError e) {
            throw new RuntimeException(
                "finkit_android: native library not found in APK", e);
        }
    }

    private Finkit() { /* no instances */ }

    /** Returns the bundled native library version, for example {@code "0.1.3"}. */
    public static native String version();

    /** Returns the JNI ABI version, used by the wrapper to refuse mismatched builds. */
    public static native int abiVersion();

    // ---- moving averages ----
    private static native double[] smaNative(double[] input, int period);
    private static native double[] emaNative(double[] input, int period);
    private static native double[] wmaNative(double[] input, int period);
    private static native double[] demaNative(double[] input, int period);
    private static native double[] temaNative(double[] input, int period);
    private static native double[] midpointNative(double[] input, int period);

    // ---- momentum ----
    private static native double[] rsiNative(double[] input, int period);
    private static native double[] rocNative(double[] input, int period);
    private static native double[] momNative(double[] input, int period);
    private static native double[] cmoNative(double[] input, int period);
    private static native double[] trixNative(double[] input, int period);

    // ---- statistics ----
    private static native double[] zscoreNative(double[] input, int period);
    private static native double[] tsfNative(double[] input, int period);
    private static native double[] linearRegNative(double[] input, int period);
    private static native double[] percentRankNative(double[] input, int period);

    /** Simple moving average. */
    public static double[] sma(double[] input, int period) { return smaNative(input, period); }
    /** Exponential moving average. */
    public static double[] ema(double[] input, int period) { return emaNative(input, period); }
    /** Weighted moving average. */
    public static double[] wma(double[] input, int period) { return wmaNative(input, period); }
    /** Double exponential moving average. */
    public static double[] dema(double[] input, int period) { return demaNative(input, period); }
    /** Triple exponential moving average. */
    public static double[] tema(double[] input, int period) { return temaNative(input, period); }
    /** Midpoint of the rolling high/low range. */
    public static double[] midpoint(double[] input, int period) { return midpointNative(input, period); }

    /** Relative Strength Index, scaled to 0..100. */
    public static double[] rsi(double[] input, int period) { return rsiNative(input, period); }
    /** Rate of change (%). */
    public static double[] roc(double[] input, int period) { return rocNative(input, period); }
    /** Momentum (delta vs N bars ago). */
    public static double[] mom(double[] input, int period) { return momNative(input, period); }
    /** Chande Momentum Oscillator. */
    public static double[] cmo(double[] input, int period) { return cmoNative(input, period); }
    /** Triple exponential oscillator. */
    public static double[] trix(double[] input, int period) { return trixNative(input, period); }
    /** Rolling z-score. */
    public static double[] zscore(double[] input, int period) { return zscoreNative(input, period); }
    /** Time-series forecast. */
    public static double[] tsf(double[] input, int period) { return tsfNative(input, period); }
    /** Linear regression forecast. */
    public static double[] linearReg(double[] input, int period) { return linearRegNative(input, period); }
    /** Rolling percentile rank. */
    public static double[] percentRank(double[] input, int period) { return percentRankNative(input, period); }
}
