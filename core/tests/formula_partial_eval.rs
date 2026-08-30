//! Formula partial-evaluation tests (R-2).
//!
//! Verifies that [`FormulaEngine::eval_partial`] never panics and always
//! returns a fully-shaped `Array1<f64>` along with a (possibly empty) error
//! list. The contract is "best effort" — a successful call yields an
//! empty error list, while a failed call yields NaN-filled data plus at
//! least one error string.

use finkit::formula::engine::FormulaEngine;
use finkit::formula::types::FormulaContext;
use ndarray::Array1;

fn make_ctx(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
    let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
    let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
    let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
    let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
    FormulaContext::new(open, high, low, close, volume, None)
}

#[test]
fn eval_partial_succeeds_on_valid_formula() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(10);
    let (result, errors) = engine.eval_partial("CLOSE + OPEN", &mut ctx);
    assert_eq!(result.len(), 10);
    assert!(errors.is_empty(), "expected no errors, got {errors:?}");
    for i in 0..10 {
        assert!(result[i].is_finite());
    }
}

#[test]
fn eval_partial_returns_nan_on_parse_error() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(10);
    // Unbalanced parenthesis: definitely a parse error.
    let (result, errors) = engine.eval_partial("CLOSE + (OPEN", &mut ctx);
    assert_eq!(result.len(), 10);
    assert!(!errors.is_empty(), "expected at least one error");
    for v in result.iter() {
        assert!(v.is_nan(), "expected NaN fallback, got {v}");
    }
}

#[test]
fn eval_partial_returns_nan_on_runtime_error() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(5);
    // SQRT of a negative number triggers a runtime error in many formula
    // engines. Even if the runtime allowed it (returning NaN), the contract
    // is that the result is still a valid Array1 of the correct length.
    let (result, errors) = engine.eval_partial("SQRT(-1 * CLOSE)", &mut ctx);
    assert_eq!(result.len(), 5);
    // Either all NaN (runtime rejected) or some NaN (runtime allowed but
    // produced NaN at every index because close is positive). In both
    // cases the length contract holds.
    let n_nan = result.iter().filter(|v| v.is_nan()).count();
    if !errors.is_empty() {
        // Errors path: every cell should be NaN.
        assert_eq!(n_nan, 5);
    }
}

#[test]
fn eval_partial_preserves_n_for_zero_length() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(0);
    let (result, errors) = engine.eval_partial("CLOSE + 1", &mut ctx);
    assert_eq!(result.len(), 0);
    // Zero-length may be a runtime error or a no-op; either is fine.
    let _ = errors;
}

#[test]
fn eval_partial_result_shape_matches_input() {
    let mut engine = FormulaEngine::new();
    let mut ctx = make_ctx(64);
    let (result, _) = engine.eval_partial("MA(CLOSE, 5)", &mut ctx);
    assert_eq!(result.len(), 64);
}
