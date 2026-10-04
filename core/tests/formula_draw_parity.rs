//! Draw commands: what the plan path reproduces, and what it still does not.
//!
//! A draw statement has two observable effects: the value it evaluates to, and
//! the command it appends to `FormulaContext::draw_commands` for the chart. The
//! tree path returns `FormulaValue::Scalar(0.0)` and appends a command; the plan
//! path filled `NaN` and appended nothing, so the same formula produced a
//! different returned series depending on the mode — invisible to every numeric
//! gate, because the corpus contains no draw-only case.
//!
//! The returned series is now identical (the dispatcher fills `0.0`, matching the
//! tree). The command side is **not** yet reproduced: the plan carries no text,
//! colour or command name for a draw node, and the dispatch layer has no way to
//! hand commands back to the context. That gap is enumerated below rather than
//! left implicit, and the test fails if reality drifts from the list in either
//! direction — so closing one of these formulas forces the entry to be removed.

use finkit::formula::{FormulaContext, FormulaEngine, FormulaExecutionMode};
use ndarray::Array1;

fn context(len: usize) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect()),
        Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.1).collect()),
        Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect()),
        Array1::from_vec((0..len).map(|i| 10.0 + (i % 3) as f64).collect()),
        Array1::from_vec(vec![1000.0; len]),
        None,
    )
}

fn engine(mode: FormulaExecutionMode) -> FormulaEngine {
    let mut engine = FormulaEngine::new();
    engine.set_execution_mode(mode);
    engine
}

/// Draw-only sources whose *returned series* must agree across both paths.
const DRAW_VALUE_PARITY: &[&str] = &[
    "DRAWLINE(C > O, C, C < O, C, 1)",
    "DRAWTEXT(C > O, C, \"BUY\")",
    "DRAWICON(C > O, C, 1)",
    "DRAWKLINE(H, O, L, C)",
];

#[test]
fn draw_statements_return_the_same_series_on_both_paths() {
    for source in DRAW_VALUE_PARITY {
        let mut tree_ctx = context(8);
        let tree = engine(FormulaExecutionMode::Tree)
            .eval(source, &mut tree_ctx)
            .unwrap_or_else(|error| panic!("tree path failed for `{source}`: {error}"));

        let mut plan_ctx = context(8);
        let plan = engine(FormulaExecutionMode::Plan)
            .eval(source, &mut plan_ctx)
            .unwrap_or_else(|error| panic!("plan path failed for `{source}`: {error}"));

        assert_eq!(
            tree.len(),
            plan.len(),
            "`{source}` returned different lengths"
        );
        for index in 0..tree.len() {
            assert!(
                (tree[index] - plan[index]).abs() <= 1e-12
                    || (tree[index].is_nan() && plan[index].is_nan()),
                "`{source}` index {index}: tree={} plan={}",
                tree[index],
                plan[index]
            );
        }
    }
}

/// Draw statements whose *chart commands* the plan path does not reproduce.
///
/// Every entry is a known, enumerated gap; the plan evaluates the arithmetic
/// correctly and simply publishes no `DrawCommand`.
const DRAW_COMMAND_BACKLOG: &[&str] = &[
    "DRAWLINE(C > O, C, C < O, C, 1)",
    "DRAWTEXT(C > O, C, \"BUY\")",
    "DRAWICON(C > O, C, 1)",
    "DRAWKLINE(H, O, L, C)",
];

fn draw_command_count(source: &str, mode: FormulaExecutionMode) -> usize {
    let mut ctx = context(8);
    let _ = engine(mode).eval(source, &mut ctx);
    let count = ctx.draw_commands.borrow().commands.len();
    count
}

#[test]
fn draw_command_backlog_matches_reality() {
    let mut observed = Vec::new();
    for source in DRAW_COMMAND_BACKLOG {
        let tree = draw_command_count(source, FormulaExecutionMode::Tree);
        let plan = draw_command_count(source, FormulaExecutionMode::Plan);
        assert!(
            tree > 0,
            "`{source}` is no longer a draw case on the tree path (tree={tree})"
        );
        if plan != tree {
            observed.push(*source);
        }
    }

    let mut declared = DRAW_COMMAND_BACKLOG.to_vec();
    declared.sort_unstable();
    observed.sort_unstable();
    assert_eq!(
        observed, declared,
        "the declared draw-command backlog no longer matches what the plan path publishes"
    );
}

#[test]
fn draw_values_are_zero_and_not_nan() {
    // The specific regression this guards: filling `NaN` made a draw-only
    // formula look like a failed indicator instead of a zero series, and it made
    // `tree == plan` false for the corpus's simplest chart statements.
    let mut ctx = context(4);
    let series = engine(FormulaExecutionMode::Plan)
        .eval("DRAWICON(C > O, C, 1)", &mut ctx)
        .expect("draw-only formula evaluates");
    assert!(
        series.iter().all(|value| *value == 0.0),
        "plan path returned {series:?} instead of a zero series"
    );
}
