//! Host-context parity between the tree path and the compiled plan path.
//!
//! `WINNER`/`COST` read a chip distribution and `PERIODTYPE` reads the chart
//! period — none of which can travel in the plan path's numeric input slots.
//! `FormulaKernelDispatcher` therefore carries a `HostContext`.
//!
//! The corpus gate cannot prove this works: the corpus fixtures carry no chip
//! data, so both paths would agree on `NaN` whether or not the channel existed.
//! These cases deliberately supply real host data and assert the two paths agree
//! *and* that the values are not NaN — a vacuous green is the failure mode here.

use finkit::formula::types::ChipData;
use finkit::formula::{FormulaContext, FormulaEngine};
use ndarray::Array1;

fn context(len: usize) -> FormulaContext {
    let series = |offset: f64| Array1::from_vec((0..len).map(|i| offset + i as f64).collect());
    FormulaContext::new(
        series(1.0),
        series(2.0),
        series(0.5),
        series(10.0),
        series(100.0),
        None,
    )
}

fn chip_data() -> ChipData {
    ChipData::with_data(
        vec![8.0, 9.0, 10.0, 11.0, 12.0],
        vec![0.1, 0.3, 0.6, 0.8, 1.0],
        1_000.0,
    )
}

fn assert_paths_agree(source: &str, ctx: &mut FormulaContext, label: &str, require_finite: bool) {
    let mut engine = FormulaEngine::new();
    let tree = engine
        .eval(source, ctx)
        .unwrap_or_else(|error| panic!("tree path failed for {label}: {error}"));
    let plan = engine
        .eval_plan(source, ctx)
        .unwrap_or_else(|error| panic!("plan path failed for {label}: {error}"));

    assert_eq!(tree.len(), plan.len(), "{label}: length mismatch");
    if require_finite {
        assert!(
            tree.iter().any(|value| value.is_finite()),
            "{label}: the tree path produced no finite value, so this case proves nothing"
        );
    }
    for index in 0..tree.len() {
        let expected = tree[index];
        let actual = plan[index];
        assert!(
            expected == actual || (expected.is_nan() && actual.is_nan()),
            "{label}: diverged at bar {index}: tree={expected} plan={actual}"
        );
    }
}

#[test]
fn winner_matches_between_tree_and_plan_when_chip_data_is_supplied() {
    let mut ctx = context(16).with_chip_data(chip_data());
    assert_paths_agree("WINNER(CLOSE)", &mut ctx, "WINNER(CLOSE)", true);
}

#[test]
fn cost_matches_between_tree_and_plan_when_chip_data_is_supplied() {
    // `COST(50)` alone has no series operand, so the plan cannot infer an
    // execution length; `CLOSE * 0 + 50` binds a series while keeping the
    // argument at exactly 50.
    let mut ctx = context(16).with_chip_data(chip_data());
    assert_paths_agree("COST(CLOSE * 0 + 50)", &mut ctx, "COST(50)", true);
}

#[test]
fn periodtype_matches_between_tree_and_plan() {
    // `with_chip_data` is not involved: this one only needs the period type.
    let mut ctx = context(16);
    ctx.period_type = 2;

    // Two things shape this source. The corpus form is `PERIODTYPE()`, not a
    // bare identifier — the parser treats a bare name as a variable. And the
    // `+ CLOSE * 0` operand is required because a formula with no series input
    // cannot give the plan an execution length; that is a pre-existing plan-path
    // constraint, not something the host context introduces.
    let source = "PERIODTYPE() + CLOSE * 0";
    let mut engine = FormulaEngine::new();
    let tree = engine.eval(source, &mut ctx).expect("tree path");
    let plan = engine.eval_plan(source, &mut ctx).expect("plan path");

    assert!(
        tree.iter().all(|value| *value == 2.0),
        "the tree path must report the host period, got {tree:?}"
    );
    assert!(
        plan.iter().all(|value| *value == 2.0),
        "the plan path must see the same host period, got {plan:?}"
    );
}

#[test]
fn host_dependent_functions_degrade_to_nan_without_host_data() {
    // Without chip data both paths must agree on NaN rather than one path
    // erroring while the other returns values.
    let mut ctx = context(8);
    assert_paths_agree("WINNER(CLOSE)", &mut ctx, "WINNER without chip data", false);

    let mut engine = FormulaEngine::new();
    let tree = engine.eval("WINNER(CLOSE)", &mut ctx).expect("tree path");
    assert!(
        tree.iter().all(|value| value.is_nan()),
        "without chip data WINNER must be NaN, got {tree:?}"
    );
}

#[test]
fn refdate_matches_between_tree_and_plan() {
    let mut ctx = context(16);
    assert_paths_agree("REFDATE(CLOSE, 5)", &mut ctx, "REFDATE(CLOSE, 5)", true);
}
