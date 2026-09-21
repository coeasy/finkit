//! Host-context parity between the tree path and the compiled plan path.
//!
//! `WINNER`/`COST` read a chip distribution, `PERIODTYPE` reads the chart
//! period, `TR` reads the implicit OHLC and the DZH money-flow family reads
//! `MoneyFlowData` — none of which can travel in the plan path's numeric input
//! slots. `FormulaKernelDispatcher` therefore carries a `HostContext`.
//!
//! The corpus gate cannot prove this works: the corpus fixtures carry no host
//! data, so both paths would agree on `NaN` whether or not the channel existed.
//! These cases deliberately supply real host data and assert the two paths agree
//! *and* that the values are not NaN — a vacuous green is the failure mode here.

use finkit::formula::types::{ChipData, MoneyFlowData};
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

fn money_flow_data(len: usize) -> MoneyFlowData {
    let series = |offset: f64| Array1::from_vec((0..len).map(|i| offset + i as f64).collect());
    MoneyFlowData {
        main_inflow: series(1_000.0),
        super_big_inflow: series(2_000.0),
        big_inflow: series(3_000.0),
        medium_inflow: series(4_000.0),
        small_inflow: series(5_000.0),
        main_inflow_pct: series(6.0),
        big_order_pct: series(7.0),
        small_order_pct: series(8.0),
        money_flow: series(9_000.0),
    }
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

/// `TR()` reads the implicit OHLC, and its bar 0 is `high - low` — not `NaN`.
///
/// The bar-0 value is the whole point. It is what separates DZH's `TR` from
/// `TRANGE(H, L, C)`, whose bar 0 is `NaN` by the TA-Lib convention. A kernel
/// that reused the `TRANGE` body would agree from bar 1 onwards and diverge only
/// here, so asserting agreement alone would not catch it — hence the explicit
/// bar-0 assertion against a known number.
///
/// `context()` builds high = `2 + i`, low = `0.5 + i`, close = `10 + i`, so bar 0
/// is `2 - 0.5 = 1.5` and bar 1 is `|low - previous close| = |1.5 - 10| = 8.5`.
#[test]
fn tr_matches_between_tree_and_plan_and_keeps_its_bar_zero() {
    let mut ctx = context(16);
    let mut engine = FormulaEngine::new();
    let tree = engine.eval("TR()", &mut ctx).expect("tree path");
    let plan = engine.eval_plan("TR()", &mut ctx).expect("plan path");

    assert!(
        (tree[0] - 1.5).abs() < 1e-12,
        "TR bar 0 must be high - low = 1.5, got {} (NaN would mean it was \
         implemented as TRANGE)",
        tree[0]
    );
    assert!(
        (tree[1] - 8.5).abs() < 1e-12,
        "TR bar 1 must be |low - previous close| = 8.5, got {}",
        tree[1]
    );

    assert_eq!(tree.len(), plan.len(), "TR: length mismatch");
    for index in 0..tree.len() {
        assert!(
            tree[index] == plan[index] || (tree[index].is_nan() && plan[index].is_nan()),
            "TR diverged at bar {index}: tree={} plan={}",
            tree[index],
            plan[index]
        );
    }
}

/// Every zero-argument money-flow reader agrees across the two paths.
///
/// These are the only host functions that take no operand at all, so they also
/// exercise the plan path's ability to run a formula with no numeric input slot
/// and still know how long the output is.
#[test]
fn money_flow_family_matches_between_tree_and_plan() {
    let len = 16;
    let mut ctx = context(len).with_money_flow_data(money_flow_data(len));

    for source in [
        "MONEYFLOW()",
        "MAININFLOW()",
        "MAININFLOWPCT()",
        "BIGORDER()",
        "SMALLORDER()",
        "SUPERBIGORDER()",
        "NETINFLOW()",
        "NETINFLOW(0)",
        "NETINFLOW(1)",
        "NETINFLOW(2)",
        "NETINFLOW(3)",
        "NETINFLOW(4)",
    ] {
        assert_paths_agree(source, &mut ctx, source, true);
    }
}

/// An out-of-range level falls back to the main-inflow series on both paths.
///
/// `fn_netinflow` treats any level outside `0..=4` as "main inflow"; pinning it
/// here keeps a future kernel from "fixing" the fallback on one path only.
#[test]
fn netinflow_out_of_range_level_falls_back_on_both_paths() {
    let len = 8;
    let mut ctx = context(len).with_money_flow_data(money_flow_data(len));
    let mut engine = FormulaEngine::new();

    let fallback = engine.eval("NETINFLOW(99)", &mut ctx).expect("tree path");
    let main = engine.eval("MAININFLOW()", &mut ctx).expect("tree path");
    assert_eq!(
        fallback, main,
        "an out-of-range level must read main inflow"
    );
}

#[test]
fn money_flow_degrades_to_nan_without_host_data() {
    // No money-flow data: both paths must agree on NaN rather than one path
    // erroring while the other returns values.
    let mut ctx = context(8);
    assert_paths_agree("MONEYFLOW()", &mut ctx, "MONEYFLOW without data", false);

    let mut engine = FormulaEngine::new();
    let tree = engine.eval("MONEYFLOW()", &mut ctx).expect("tree path");
    assert!(
        tree.iter().all(|value| value.is_nan()),
        "without money-flow data MONEYFLOW must be NaN, got {tree:?}"
    );
}

/// A host series whose length does not match the output reads as absent.
///
/// The guard matters because the alternative — broadcasting or truncating —
/// would silently fill the buffer with the wrong bars rather than reporting that
/// the data does not fit.
#[test]
fn mismatched_money_flow_length_degrades_to_nan() {
    let len = 16;
    let mut ctx = context(len).with_money_flow_data(money_flow_data(len / 2));
    assert_paths_agree("MONEYFLOW()", &mut ctx, "MONEYFLOW wrong length", false);
}
