//! `for` loop unrolling and `INDEX` semantics on the compiled plan path.
//!
//! The corpus gate in `formula_plan_differential.rs` proves tree and plan agree
//! for the Pine corpus. These tests pin the parts the corpus cannot:
//!
//! - that an unrolled loop produces a *real* number, not a vacuous agreement
//!   between two NaNs (the corpus fixtures make an empty loop body agree).
//! - that a loop which cannot be unrolled fails loudly instead of being dropped.
//! - that `array[index]` gathers and broadcasts rather than shifting.

use finkit::formula::pine::{map_pine_to_alphata, parse_pine};
use finkit::formula::{
    unified_formula_executor, AstNode, FormulaContext, FormulaEngine, FormulaHotPlan,
};
use ndarray::Array1;

/// Deterministic synthetic OHLCV.
fn context(n: usize) -> FormulaContext {
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rng = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (u64::MAX as f64)
    };
    let mut close = Vec::with_capacity(n);
    let mut c = 100.0;
    for _ in 0..n {
        c += (rng() - 0.5) * 4.0;
        close.push(c);
    }
    let open = close.clone();
    let high: Vec<f64> = close.iter().map(|v| v + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 1.0).collect();
    let volume: Vec<f64> = (0..n).map(|i| 1000.0 + i as f64).collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close.clone()),
        Array1::from_vec(volume),
        None,
    )
}

fn pine_ast(source: &str) -> AstNode {
    let pine = parse_pine(source).unwrap_or_else(|e| panic!("parse_pine: {e}"));
    map_pine_to_alphata(&pine).unwrap_or_else(|e| panic!("map_pine_to_alphata: {e}"))
}

/// Run one AST through the compiled plan path, binding every declared input.
fn plan_values(ast: &AstNode, ctx: &FormulaContext) -> Result<Array1<f64>, String> {
    let plan = FormulaHotPlan::compile(ast).map_err(|e| format!("compile: {e}"))?;
    let mut slots: Vec<Option<&[f64]>> = vec![None; plan.hot().input_layout().len()];
    for binding in plan.input_bindings() {
        let values = ctx
            .get_data(binding.name())
            .ok_or_else(|| format!("missing input series `{}`", binding.name()))?;
        slots[binding.slot().0] = Some(values);
    }
    let mut inputs = Vec::with_capacity(slots.len());
    for (index, slot) in slots.into_iter().enumerate() {
        inputs.push(slot.ok_or_else(|| format!("input slot {index} was never bound"))?);
    }
    let mut executor = unified_formula_executor(&plan);
    let result = executor
        .execute(&inputs)
        .map_err(|e| format!("execute: {e}"))?;
    result
        .values
        .into_iter()
        .next()
        .map(Array1::from_vec)
        .ok_or_else(|| "plan produced no output series".to_string())
}

/// The loop body must actually run: the total has to be the real sum, not the
/// seed value an empty body would leave behind.
#[test]
fn unrolled_loop_accumulates_a_real_total() {
    let ast = pine_ast(
        "//@version=5\nindicator(\"t\")\nlookback = 4\ntotal = 0.0\nfor i = 0 to lookback - 1\n    total := total + close[i]\nplot(total, \"T\")\n",
    );
    let mut ctx = context(40);
    let reference = FormulaEngine::new().eval_ast(&ast, &mut ctx).unwrap();
    let candidate = plan_values(&ast, &ctx).expect("plan path must run an unrolled loop");

    assert_eq!(reference.len(), candidate.len());
    let expected: f64 = (0..4).map(|i| ctx.close[i]).sum();
    for index in 0..reference.len() {
        assert!(
            (reference[index] - candidate[index]).abs() < 1e-9,
            "index {index}: tree={} plan={}",
            reference[index],
            candidate[index]
        );
        // Not vacuous: an empty loop body would leave `total` at its 0.0 seed.
        assert!(
            (candidate[index] - expected).abs() < 1e-9,
            "index {index}: expected the sum of close[0..4]={expected}, got {}",
            candidate[index]
        );
    }
}

/// `array[index]` is a gather with broadcast, not a `REF` shift: a constant
/// index yields one historical element repeated across the whole series.
#[test]
fn index_access_gathers_and_broadcasts_instead_of_shifting() {
    let ast = pine_ast("//@version=5\nindicator(\"t\")\nplot(close[3], \"C\")\n");
    let mut ctx = context(40);
    let reference = FormulaEngine::new().eval_ast(&ast, &mut ctx).unwrap();
    let candidate = plan_values(&ast, &ctx).expect("plan path must run INDEX");

    let expected = ctx.close[3];
    for index in 0..candidate.len() {
        assert!(
            (candidate[index] - expected).abs() < 1e-9,
            "index {index}: expected a constant broadcast of close[3]={expected}, got {}",
            candidate[index]
        );
        assert!(
            (reference[index] - candidate[index]).abs() < 1e-9,
            "index {index}: tree={} plan={}",
            reference[index],
            candidate[index]
        );
    }
}

/// A loop whose bounds are not compile-time constants cannot be unrolled, and
/// must not be silently dropped: dropping it would leave every accumulator at
/// its seed value and produce a wrong number rather than an error.
#[test]
fn non_constant_loop_bounds_fail_loudly() {
    let ast = pine_ast(
        "//@version=5\nindicator(\"t\")\ntotal = 0.0\nfor i = 0 to close\n    total := total + 1.0\nplot(total, \"T\")\n",
    );
    let error = FormulaHotPlan::compile(&ast)
        .err()
        .expect("a loop with a series bound must not compile into a silent no-op");
    let message = error.to_string();
    assert!(
        message.contains("for") && message.contains("loop"),
        "expected a loop-specific compile error, got: {message}"
    );
}
