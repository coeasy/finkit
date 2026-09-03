package com.finkit;

/**
 * Owns the native K-line data handle used by {@link KlineChart}.
 *
 * <p>Instances must be closed when no longer needed.
 */
public final class KlineData implements AutoCloseable {
    private long nativeHandle;

    public KlineData(String[] dates, double[] opens, double[] highs, double[] lows,
                     double[] closes, double[] volumes) {
        if (dates == null || opens == null || highs == null || lows == null
                || closes == null || volumes == null) {
            throw new IllegalArgumentException("K-line inputs must not be null");
        }
        int length = dates.length;
        if (opens.length != length || highs.length != length || lows.length != length
                || closes.length != length || volumes.length != length) {
            throw new IllegalArgumentException("K-line input arrays must have equal lengths");
        }
        KlineChart.ensureLoaded();
        nativeHandle = KlineChart.klineDataNew(dates, opens, highs, lows, closes, volumes);
        if (nativeHandle == 0) {
            throw new IllegalStateException("Failed to allocate native K-line data");
        }
    }

    long handle() {
        if (nativeHandle == 0) {
            throw new IllegalStateException("KlineData is already closed");
        }
        return nativeHandle;
    }

    public boolean validate() {
        return nativeHandle != 0 && KlineChart.klineDataValidate(nativeHandle);
    }

    @Override
    public void close() {
        long handle = nativeHandle;
        nativeHandle = 0;
        if (handle != 0) {
            KlineChart.klineDataFree(handle);
        }
    }
}
