//! Cross-language golden test: AlphaTA core must reproduce the canonical
//! reference values published in `finkit_ffi_common::golden`.
//!
//! Because the same `golden` module is imported by every FFI binding's test
//! suite, a single regression here (or in any binding) is caught against one
//! shared source of truth. We assert only the *last* output element so the
//! check is independent of warm-up / length conventions across languages.

use finkit::math::moving_avg::{ema, sma};
use finkit_ffi_common::golden::{assert_close, EMA_GOLDEN, SMA_GOLDEN};

#[test]
fn golden_sma_matches_core() {
    for g in SMA_GOLDEN {
        let out = sma(g.input, g.period).expect("sma should succeed");
        let last = out[out.len() - 1];
        assert_close(last, g.expected_last, 1e-9);
    }
}

#[test]
fn golden_ema_matches_core() {
    for g in EMA_GOLDEN {
        let out = ema(g.input, g.period).expect("ema should succeed");
        let last = out[out.len() - 1];
        assert_close(last, g.expected_last, 1e-9);
    }
}
