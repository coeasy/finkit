package com.finkit;

/**
 * Native SVG K-line chart builder.
 *
 * <p>All native handles are released by {@link #close()}.
 */
public final class KlineChart implements AutoCloseable {
    private long nativeHandle;

    static {
        Indicators.ensureLoaded();
    }

    public KlineChart(KlineData data, String language, String title, int width, int height) {
        if (data == null || language == null || title == null) {
            throw new IllegalArgumentException("Chart inputs must not be null");
        }
        if (width <= 0 || height <= 0) {
            throw new IllegalArgumentException("Chart dimensions must be positive");
        }
        nativeHandle = klineChartNew(data.handle(), language, title, width, height);
        if (nativeHandle < 0) {
            throw new IllegalStateException("Failed to create native K-line chart");
        }
    }

    public void addMa(int[] periods) {
        ensureOpen();
        if (periods == null) {
            throw new IllegalArgumentException("periods must not be null");
        }
        klineChartAddMa(nativeHandle, periods);
    }

    public void addMacd(int fast, int slow, int signal) {
        ensureOpen();
        klineChartAddMacd(nativeHandle, fast, slow, signal);
    }

    public void addRsi(int period) {
        ensureOpen();
        klineChartAddRsi(nativeHandle, period);
    }

    public void addBoll(int period, double nbDev) {
        ensureOpen();
        klineChartAddBoll(nativeHandle, period, nbDev);
    }

    public void saveAsSvg(String path) {
        ensureOpen();
        if (path == null) {
            throw new IllegalArgumentException("path must not be null");
        }
        klineChartSaveAsSvg(nativeHandle, path);
    }

    public String toSvg() {
        ensureOpen();
        String svg = klineChartToSvg(nativeHandle);
        if (svg == null) {
            throw new IllegalStateException("Failed to render native K-line chart");
        }
        return svg;
    }

    private void ensureOpen() {
        if (nativeHandle == 0) {
            throw new IllegalStateException("KlineChart is already closed");
        }
    }

    static void ensureLoaded() {
        Indicators.ensureLoaded();
    }

    static native long klineDataNew(String[] dates, double[] opens, double[] highs,
                                    double[] lows, double[] closes, double[] volumes);
    static native void klineDataFree(long handle);
    static native boolean klineDataValidate(long handle);

    private static native long klineChartNew(long dataHandle, String language, String title,
                                              int width, int height);
    private static native void klineChartFree(long handle);
    private static native void klineChartAddMa(long handle, int[] periods);
    private static native void klineChartAddMacd(long handle, int fast, int slow, int signal);
    private static native void klineChartAddRsi(long handle, int period);
    private static native void klineChartAddBoll(long handle, int period, double nbDev);
    private static native void klineChartSaveAsSvg(long handle, String path);
    private static native String klineChartToSvg(long handle);

    @Override
    public void close() {
        long handle = nativeHandle;
        nativeHandle = 0;
        if (handle != 0) {
            klineChartFree(handle);
        }
    }
}
