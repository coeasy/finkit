package com.alphata.indicators;

/**
 * High-performance financial technical analysis for Android, backed by
 * the AlphaTA Rust core via JNI.
 *
 * <p>Call {@link #init()} once (typically from {@code Application.onCreate})
 * to load the native library, then call the static indicator methods.
 *
 * <p>All public methods are thread-safe — the underlying Rust functions
 * are pure (no shared mutable state), so the JNI overhead is dominated
 * by the {@code double[]} marshalling.
 */
public final class AlphaTA {

    static {
        try {
            System.loadLibrary("alpha_ta_android");
        } catch (UnsatisfiedLinkError e) {
            throw new RuntimeException(
                "alpha_ta_android: native library not found in APK", e);
        }
    }

    private AlphaTA() { /* no instances */ }

    /** Returns the bundled native library version, e.g. {@code "1.0.0"}. */
    public static native String version();

    /** Returns the JNI ABI version, used by the wrapper to refuse mismatched builds. */
    public static native int abiVersion();

    // ---- moving averages ----
    private static native double[] smaNative(double[] input, int period);
    private static native double[] emaNative(double[] input, int period);
    private static native double[] wmaNative(double[] input, int period);

    // ---- momentum ----
    private static native double[] rsiNative(double[] input, int period);
    private static native double[] rocNative(double[] input, int period);
    private static native double[] momNative(double[] input, int period);

    /** Simple moving average. */
    public static double[] sma(double[] input, int period) { return smaNative(input, period); }
    /** Exponential moving average. */
    public static double[] ema(double[] input, int period) { return emaNative(input, period); }
    /** Weighted moving average. */
    public static double[] wma(double[] input, int period) { return wmaNative(input, period); }
    /** Relative Strength Index, scaled to 0..100. */
    public static double[] rsi(double[] input, int period) { return rsiNative(input, period); }
    /** Rate of change (%). */
    public static double[] roc(double[] input, int period) { return rocNative(input, period); }
    /** Momentum (delta vs N bars ago). */
    public static double[] mom(double[] input, int period) { return momNative(input, period); }
}