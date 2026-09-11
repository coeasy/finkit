package com.finkit;

/** Canonical strategy, benchmark, trade and portfolio evaluation. */
public final class QuantEvaluation {
    static { NativeLoader.load(); }
    private QuantEvaluation() {}
    public static native String evaluateJson(String requestJson);
}
