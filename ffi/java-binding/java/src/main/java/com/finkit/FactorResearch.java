package com.finkit;

/** Versioned panel-aware factor research backed by the canonical Rust engine. */
public final class FactorResearch {
    static { NativeLoader.load(); }
    private FactorResearch() {}
    public static native String factorStudyJson(String requestJson);
}
