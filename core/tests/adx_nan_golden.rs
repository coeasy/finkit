//! NaN / zero-band golden semantics for the ADX family (V5 Batch 1 ① + ④).
//!
//! These tests are the golden contract for the `adx_into` merge: the public
//! indicator kernel (`indicators::momentum::adx_into`) and the canonical
//! TA-Lib-compat kernel (`math::kernels::compat::adx_into`, driving
//! `AdxState`/`DmiState`) must agree **bit for bit** on every input — clean
//! OHLC, NaN-bearing OHLC, and degenerate flat OHLC. Before the merge the two
//! implementations disagreed on NaN inputs (the public one propagated NaN via
//! a comparison-style true range; the canonical one absorbs it via `f64::max`,
//! matching TA-Lib's `fmax`), and on denominators in the `(1e-15, 1e-8)` band
//! (hand-picked `1e-15` guard vs TA-Lib's `TA_IS_ZERO` 1e-8 bandwidth).

use finkit::indicators::momentum::adx_into as public_adx_into;
use finkit::math::kernels::adx_into as canonical_adx_into;
use finkit::utils::{is_zero, true_range};

/// Bit-for-bit equality with NaN == NaN (the lookback region is NaN on both
/// sides; that is agreement, not a mismatch).
fn assert_bitwise_equal(left: &[f64], right: &[f64]) {
    assert_eq!(left.len(), right.len());
    for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
        assert_eq!(
            l.to_bits(),
            r.to_bits(),
            "bit mismatch at index {index}: {l} vs {r}"
        );
    }
}

/// Deterministic trending OHLC (no NaN) with enough bars for period 14.
fn trending_ohlc(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let high: Vec<f64> = (0..len)
        .map(|i| 100.0 + i as f64 * 0.5 + (i % 7) as f64 * 0.1)
        .collect();
    let low: Vec<f64> = (0..len)
        .map(|i| high[i] - 2.0 - (i % 5) as f64 * 0.2)
        .collect();
    let close: Vec<f64> = (0..len).map(|i| (high[i] + low[i]) * 0.5).collect();
    (high, low, close)
}

#[test]
fn public_adx_matches_canonical_kernel_bit_for_bit_on_clean_data() {
    let (high, low, close) = trending_ohlc(200);
    for period in [7usize, 14, 20] {
        let mut via_public = vec![f64::NAN; 200];
        let mut via_canonical = vec![f64::NAN; 200];
        public_adx_into(&high, &low, &close, period, &mut via_public).unwrap();
        canonical_adx_into(&high, &low, &close, period, &mut via_canonical).unwrap();
        assert_bitwise_equal(&via_public, &via_canonical);
    }
}

#[test]
fn public_adx_matches_canonical_kernel_bit_for_bit_on_nan_data() {
    // The V5 R1-1 divergence scenario: NaN injected into the OHLC stream.
    // Before the merge, the public kernel propagated the NaN into ADX while
    // the canonical kernel (TA-Lib `fmax` semantics) absorbed it — same
    // input, two answers. Now both behave like TA-Lib: NaN is absorbed as
    // long as the surviving max candidates are finite.
    let (mut high, mut low, mut close) = trending_ohlc(200);
    high[37] = f64::NAN;
    low[59] = f64::NAN;
    close[73] = f64::NAN;
    high[101] = f64::NAN;
    low[101] = f64::NAN;

    let mut via_public = vec![f64::NAN; 200];
    let mut via_canonical = vec![f64::NAN; 200];
    public_adx_into(&high, &low, &close, 14, &mut via_public).unwrap();
    canonical_adx_into(&high, &low, &close, 14, &mut via_canonical).unwrap();
    assert_bitwise_equal(&via_public, &via_canonical);

    // And the canonical ATR family shares the same true-range semantics, so
    // the whole OHLC family answers NaN the same way.
    let tr = true_range(f64::NAN, 10.0, 9.0);
    assert!(
        tr == 1.0,
        "true range with a NaN high must absorb it (TA-Lib fmax), got {tr}"
    );
}

#[test]
fn flat_market_hits_the_zero_guard_and_falls_back_to_zero() {
    // Every bar identical: every true range and directional movement is
    // exactly 0.0, so the smoothed denominators sit far inside the TA_IS_ZERO
    // band. The recorded fallback contract: DI and DX (hence ADX) are 0.0.
    let high = vec![100.0; 64];
    let low = vec![100.0; 64];
    let close = vec![100.0; 64];
    let mut out = vec![f64::NAN; 64];
    public_adx_into(&high, &low, &close, 14, &mut out).unwrap();
    for value in out.iter().skip(2 * 14 - 1) {
        assert!(
            *value == 0.0,
            "ADX on a flat market must be the 0.0 fallback, got {value}"
        );
    }
}

#[test]
fn is_zero_is_the_ta_lib_band_predicate() {
    // `TA_IS_ZERO(v) = (v > -1e-8) && (v < 1e-8)` — exactly positive zero,
    // negative zero, and anything strictly inside the band.
    assert!(is_zero(0.0));
    assert!(is_zero(-0.0));
    assert!(is_zero(1e-9));
    assert!(is_zero(-1e-9));
    assert!(
        !is_zero(1e-8),
        "the band is open at 1e-8 (TA-Lib: strict <)"
    );
    assert!(!is_zero(-1e-8));
    assert!(!is_zero(1e-7));
    assert!(!is_zero(f64::NAN), "NaN is not zero");
}
