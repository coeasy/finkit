//! `FormulaExecutionMode` is a real switch, not a hint.
//!
//! Parity alone would not prove that: with the differential gates green on both
//! corpora, a switch wired to nothing would also produce identical numbers.
//! So these tests pin the two things that actually differ between the paths,
//! and the parity that must hold for everything else.

use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode};
use ndarray::Array1;

fn context(n: usize) -> FormulaContext {
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
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
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

const FORMULAS: &[&str] = &[
    "MA(CLOSE, 5)",
    "EMA(CLOSE, 12)",
    "RSI(CLOSE, 14)",
    "IF(CLOSE > 0, 1, 0)",
    "(CLOSE + OPEN) / 2",
    "HHV(HIGH, 10)",
];

#[test]
fn tree_is_the_default() {
    assert_eq!(
        FormulaEngine::new().execution_mode(),
        FormulaExecutionMode::Tree,
        "the reference path must stay the default until flipping it is a deliberate release decision"
    );
}

#[test]
fn both_modes_agree_on_the_primary_series() {
    for source in FORMULAS {
        let ctx = context(64);
        let tree = FormulaEngine::new()
            .eval(source, &mut ctx.clone())
            .unwrap_or_else(|e| panic!("tree failed on {source}: {e}"));
        let plan = FormulaEngine::new()
            .with_execution_mode(FormulaExecutionMode::Plan)
            .eval(source, &mut ctx.clone())
            .unwrap_or_else(|e| panic!("plan failed on {source}: {e}"));

        assert_eq!(tree.len(), plan.len(), "length mismatch on {source}");
        for index in 0..tree.len() {
            let matches = (tree[index] - plan[index]).abs() < 1e-9
                || (tree[index].is_nan() && plan[index].is_nan());
            assert!(
                matches,
                "{source} index {index}: tree={} plan={}",
                tree[index], plan[index]
            );
        }
    }
}

/// The switch must actually move work onto the other path.
///
/// The tree-walker writes assignment results back into `ctx.variables`; the
/// plan path reads inputs and returns outputs without touching it. Observing
/// that difference is what distinguishes a wired switch from a no-op — parity
/// tests cannot, because both paths now agree numerically.
#[test]
fn the_mode_changes_which_path_runs() {
    let source = "X:MA(CLOSE, 5);";

    let mut tree_ctx = context(32);
    FormulaEngine::new()
        .eval(source, &mut tree_ctx)
        .expect("tree path must evaluate");
    let tree_wrote_variables = !tree_ctx.variables.is_empty();

    let mut plan_ctx = context(32);
    FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval(source, &mut plan_ctx)
        .expect("plan path must evaluate");
    let plan_wrote_variables = !plan_ctx.variables.is_empty();

    assert!(
        tree_wrote_variables,
        "the tree path is expected to publish assignments into ctx.variables; \
         if it stops, this test no longer discriminates the two paths"
    );
    assert!(
        !plan_wrote_variables,
        "the plan path must not mutate ctx.variables — if it does, the engine is \
         still running the tree-walker in Plan mode"
    );
}

/// Plan mode must reject a formula it cannot lower rather than quietly
/// producing a tree-path answer.
#[test]
fn plan_mode_fails_loudly_instead_of_falling_back() {
    let ctx = context(32);
    // `WHILE` has no acyclic lowering, so this is compile-time unsupported.
    let source = "X:0; WHILE CLOSE > 0 X:X+1;";

    let plan = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval(source, &mut ctx.clone());

    assert!(
        plan.is_err(),
        "Plan mode must not silently succeed on a formula it cannot lower"
    );
}

/// The switch is reachable through the dialect-aware entry point too, since
/// that is what FFI callers use.
#[test]
fn the_switch_applies_to_dialect_entry_points() {
    let ctx = context(32);
    let tree = FormulaEngine::new()
        .eval_with_dialect("MA(CLOSE, 5)", FormulaDialect::TongDaXin, &mut ctx.clone())
        .expect("tree");
    let plan = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval_with_dialect("MA(CLOSE, 5)", FormulaDialect::TongDaXin, &mut ctx.clone())
        .expect("plan");

    assert_eq!(tree.len(), plan.len());
    for index in 0..tree.len() {
        let matches = (tree[index] - plan[index]).abs() < 1e-9
            || (tree[index].is_nan() && plan[index].is_nan());
        assert!(
            matches,
            "index {index}: tree={} plan={}",
            tree[index], plan[index]
        );
    }
}
