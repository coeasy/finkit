//! Runtime CPU feature detection for the SIMD dispatch layer.

// ============================================================================
// Runtime SIMD capability detection
// ============================================================================

/// Returns `true` if the current CPU supports AVX2 instructions.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[inline]
pub fn has_avx2() -> bool {
    is_x86_feature_detected!("avx2")
}

#[cfg(not(all(feature = "std", target_arch = "x86_64")))]
#[inline]
pub fn has_avx2() -> bool {
    false
}

/// Returns `true` if the current CPU supports SSE4.1 instructions.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[inline]
pub fn has_sse41() -> bool {
    is_x86_feature_detected!("sse4.1")
}

#[cfg(not(all(feature = "std", target_arch = "x86_64")))]
#[inline]
pub fn has_sse41() -> bool {
    false
}

/// Returns `true` if the current CPU supports FMA instructions.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[inline]
pub fn has_fma() -> bool {
    is_x86_feature_detected!("fma")
}

#[cfg(not(all(feature = "std", target_arch = "x86_64")))]
#[inline]
pub fn has_fma() -> bool {
    false
}

/// Returns a bitmask of detected SIMD capabilities for diagnostic use.
/// Bit 0 = SSE4.1, Bit 1 = AVX2, Bit 2 = FMA.
#[inline]
pub fn simd_capability_flags() -> u32 {
    let mut flags = 0u32;
    if has_sse41() {
        flags |= 1;
    }
    if has_avx2() {
        flags |= 2;
    }
    if has_fma() {
        flags |= 4;
    }
    flags
}
