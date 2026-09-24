//! External-consumer smoke test for the public `multi_period_resonance` API.
//!
//! `scripts/check_orphan_modules.py` only counts `crate::<module>::` /
//! `finkit::<module>::` references as real callers, so an in-crate
//! `#[cfg(test)] mod tests { use super::*; }` block is invisible to it. This
//! file exercises the surface through the public `finkit::` path so the module
//! is provably reachable from outside the crate (no orphan logic) and the
//! allowlist entry can move from `orphan` to `test-only`.

use finkit::multi_period_resonance::{mtf_majority, mtf_resonance, mtf_trend_filter, mtf_weighted};
use finkit::patterns::Signal;
use ndarray::Array1;

fn sig(values: &[i32]) -> Array1<Signal> {
    Array1::from(values.to_vec())
}

#[test]
fn external_mtf_resonance_is_reachable() {
    let daily = sig(&[0, 100, 0, 0, -100]);
    // bar 1: both bullish (>= 2 agree) -> 100; bar 2: only one bullish -> 0.
    let weekly = sig(&[0, 100, 100, 0, 0]);
    let out = mtf_resonance(&[&daily, &weekly], 2);
    assert_eq!(out[1], 100);
    assert_eq!(out[2], 0);
    assert_eq!(out[4], 0);
}

#[test]
fn external_mtf_majority_is_reachable() {
    let a = sig(&[100, 100, 0, -100, -100]);
    let b = sig(&[100, 0, 0, 0, -100]);
    let c = sig(&[0, 0, 0, 0, 0]);
    let out = mtf_majority(&[&a, &b, &c]);
    assert_eq!(out[0], 100);
    assert_eq!(out[3], -100);
}

#[test]
fn external_mtf_trend_filter_is_reachable() {
    let higher = sig(&[0, 100, 100, 0, -100]);
    let lower = sig(&[100, 100, 100, 100, 100]);
    let out = mtf_trend_filter(&higher, &lower);
    assert_eq!(out[1], 100);
    assert_eq!(out[4], -100);
    assert_eq!(out[0], 0);
}

#[test]
fn external_mtf_weighted_is_reachable() {
    let a = sig(&[100, 0, -100]);
    let b = sig(&[-100, 0, 100]);
    let out = mtf_weighted(&[&a, &b], &[0.9, 0.1]);
    assert_eq!(out[0], 100);
    assert_eq!(out[1], 0);
    assert_eq!(out[2], -100);
}
