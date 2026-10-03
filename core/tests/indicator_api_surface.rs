//! Compile-time guard for the public zero-copy (`_into`) indicator surface.
//!
//! # Why this target exists
//!
//! The `_into` family is the caller-owned-buffer half of the public API: every
//! entry point in it writes into a `&mut [f64]` instead of allocating, which is
//! what the FFI layers, the formula runtime and the benchmark harnesses bind
//! to. Unlike the allocating half it has no other consumer inside this crate,
//! because the internal callers reach for the canonical maths kernels directly
//! (`crate::math::...`) rather than for the `indicators::` wrappers.
//!
//! That asymmetry produced a silent API break. During the kernel-unification
//! pass a duplicated fused implementation was deleted from
//! `indicators::volume` and the allocation path was rewired onto the kernel —
//! but the public name `adosc_into` was not re-attached to anything. Every
//! gate stayed green:
//!
//! * `cargo check --workspace --all-targets` cannot notice a *removed*
//!   `pub fn` that nothing inside the workspace calls;
//! * the sibling unit test in `indicators/volume.rs` kept passing because it
//!   had been rewritten to call `math::volume_kernels::adosc_into` directly, so
//!   the test that *looked* like it covered the public entry point was
//!   exercising the kernel instead;
//! * the generated SSOT snapshot (`docs/generated/indicators.md`) faithfully
//!   recorded the new state, because a snapshot has no opinion about what the
//!   surface should be.
//!
//! The lesson is that "nothing calls it" is not evidence that "nothing needs
//! it". Downstream bindings and out-of-tree users are callers too, and they are
//! invisible to `cargo check`.
//!
//! # What this target guarantees
//!
//! Each `assert_paths_exist!` entry names a path that must resolve. Deleting or
//! renaming any of them fails *compilation* of this target, which makes the
//! removal a deliberate, reviewable decision rather than a side effect of an
//! unrelated refactor.
//!
//! The guard is intentionally one-directional: it pins the names that must keep
//! existing, and says nothing about new ones. Adding an entry point therefore
//! does not require touching this file, so the guard cannot rot into a
//! maintenance chore and get deleted.
//!
//! # A note on the root-level names
//!
//! `indicators::mod.rs` re-exports `volume::*` and then re-exports
//! `math::volume_kernels::{ad, adosc, obv}` explicitly. An explicit re-export
//! shadows a glob re-export, so the *root* paths `indicators::ad` and
//! `indicators::adosc` resolve to the kernels while
//! `indicators::volume::ad` resolves to the module wrapper. Both are public and
//! both are used, so the root paths are pinned separately below — this is the
//! exact seam where `adosc_into` fell through.

/// Asserts that every listed path resolves, at compile time.
///
/// A bare `let _ = <path>;` forces name resolution without coercing the item to
/// a function pointer, so entries need no signature and stay valid if a
/// parameter list gains an argument.
macro_rules! assert_paths_exist {
    ($($item:path),* $(,)?) => {
        $( let _ = $item; )*
    };
}

#[test]
fn every_documented_zero_copy_entry_point_is_still_reachable() {
    assert_paths_exist!(
        // cycle
        finkit::indicators::cycle::ht_dcperiod_into,
        finkit::indicators::cycle::ht_dcphase_into,
        finkit::indicators::cycle::ht_phasor_into,
        finkit::indicators::cycle::ht_sine_into,
        finkit::indicators::cycle::ht_trendline_into,
        finkit::indicators::cycle::ht_trendmode_into,
        // momentum
        finkit::indicators::momentum::rsi_into,
        finkit::indicators::momentum::stoch_into,
        finkit::indicators::momentum::adx_from_di_into,
        finkit::indicators::momentum::plus_di_fast_into,
        finkit::indicators::momentum::minus_di_fast_into,
        finkit::indicators::momentum::aroon_into,
        finkit::indicators::momentum::cci_source_into,
        finkit::indicators::momentum::cci_into,
        finkit::indicators::momentum::willr_into,
        finkit::indicators::momentum::willr14_into,
        finkit::indicators::momentum::cmo_fast_into,
        finkit::indicators::momentum::dx_into,
        finkit::indicators::momentum::trix_into,
        finkit::indicators::momentum::adxr_into,
        finkit::indicators::momentum::macdfix_into,
        finkit::indicators::momentum::stochf_into,
        finkit::indicators::momentum::stochrsi_into,
        finkit::indicators::momentum::ultosc_into,
        finkit::indicators::momentum::macd_into,
        finkit::indicators::momentum::macd_fast_into,
        finkit::indicators::momentum::macd_line_into,
        finkit::indicators::momentum::adx_into,
        finkit::indicators::momentum::mom_into,
        finkit::indicators::momentum::roc_into,
        // overlap
        finkit::indicators::overlap::midpoint_into,
        finkit::indicators::overlap::midpoint14_into,
        finkit::indicators::overlap::midprice_into,
        finkit::indicators::overlap::midprice14_into,
        finkit::indicators::overlap::sar_with_factors_into,
        finkit::indicators::overlap::sarext_sar_into,
        finkit::indicators::overlap::mama_into,
        finkit::indicators::overlap::t3_into,
        finkit::indicators::overlap::bbands_into,
        finkit::indicators::overlap::dema_into,
        finkit::indicators::overlap::tema_into,
        // price_transform
        finkit::indicators::price_transform::wclprice_into,
        finkit::indicators::price_transform::avgprice_into,
        finkit::indicators::price_transform::medprice_into,
        finkit::indicators::price_transform::typprice_into,
        // statistics
        finkit::indicators::statistics::zscore_into,
        // volatility
        finkit::indicators::volatility::atr_into,
        finkit::indicators::volatility::natr_into,
        finkit::indicators::volatility::trange_into,
        finkit::indicators::volatility::trange_dzh_into,
        // volume
        finkit::indicators::volume::ad_into,
        finkit::indicators::volume::adosc_into,
        finkit::indicators::volume::obv_into,
    );
}

/// The root re-export seam, pinned separately from the module paths above.
///
/// `adosc_into` is the name that this guard was written for: it is listed on
/// both sides on purpose, so losing either path is caught.
#[test]
fn root_re_exports_keep_resolving_to_something_callable() {
    assert_paths_exist!(
        finkit::indicators::ad,
        finkit::indicators::adosc,
        finkit::indicators::obv,
        finkit::indicators::ad_into,
        finkit::indicators::adosc_into,
        finkit::indicators::obv_into,
        // Explicit re-exports that shadow a same-named glob member.
        finkit::indicators::trange,
        finkit::indicators::trange_into,
    );
}

/// The restored entry point must remain *usable*, not merely nameable.
///
/// A resolved path proves the item exists; it does not prove the wrapper still
/// forwards. This exercises the public `indicators::volume::adosc_into` and
/// checks it against the allocating entry point, so a future refactor that
/// leaves the name in place but breaks the forwarding fails here rather than
/// downstream.
#[test]
fn the_public_adosc_into_wrapper_still_agrees_with_adosc() {
    let high = [10.0, 12.0, 11.0, 14.0, 15.0, 13.0, 16.0, 17.0, 18.0];
    let low = [8.0, 10.0, 9.0, 12.0, 13.0, 11.0, 14.0, 15.0, 16.0];
    let close = [9.0, 11.0, 10.0, 13.0, 14.0, 12.0, 15.0, 16.0, 17.0];
    let volume = [
        100.0, 120.0, 110.0, 130.0, 140.0, 125.0, 150.0, 160.0, 170.0,
    ];

    let expected = finkit::indicators::volume::adosc(&high, &low, &close, &volume, 3, 5)
        .expect("allocating ADOSC accepts a well-formed series");
    let mut actual = [0.0; 9];
    finkit::indicators::volume::adosc_into(&high, &low, &close, &volume, 3, 5, &mut actual)
        .expect("zero-copy ADOSC accepts the same series");

    // The warm-up region reports "no value yet" as NaN on both paths, and a
    // tolerance comparison cannot express that agreement (`NaN - NaN` is NaN).
    for (index, (lhs, rhs)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(
            (lhs.is_nan() && rhs.is_nan()) || (lhs - rhs).abs() <= 1e-12,
            "bar {index}: {lhs} != {rhs}"
        );
    }
}
