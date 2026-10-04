//! String literals on the compiled plan path.
//!
//! The tree path evaluates `AstNode::StringLit` by appending the text to
//! `FormulaContext::string_table` and yielding the new slot's index; several
//! functions (`EM_REF`, `BLOCKDATA`, `BLOCKINDEX`, `BLOCKAVG`) take that index
//! as their argument and read the text back. The plan path used to lower the
//! node to a bare `STRING_LITERAL` marker that carried no text and had no
//! kernel, so *any* formula containing a string literal failed at dispatch —
//! which is what kept `FormulaExecutionMode::Plan` from being the default.
//!
//! These tests pin the repaired contract:
//!
//! * the literal texts reach `ctx.string_table` in the same order on both paths,
//!   including when the caller pre-populated the table (the index offset case),
//! * the emitted index is the absolute table position, and
//! * the string-consuming functions the plan dispatcher still cannot run are
//!   listed explicitly, one by one, instead of being silently absent.

use finkit::formula::{
    parse_formula, unified_formula_executor, FormulaContext, FormulaEngine, FormulaExecutionMode,
    FormulaHotPlan,
};
use ndarray::Array1;
use std::collections::BTreeMap;

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

/// Sources whose literals survive lowering, paired with the table both paths
/// must build.
const LITERAL_CASES: &[(&str, &[&str])] = &[
    ("EM_REF(\"IDX\", 1)", &["IDX"]),
    ("BLOCKINDEX(\"BK\")", &["BK"]),
    ("BLOCKAVG(\"BK\")", &["BK"]),
    ("EM_REF(\"A\", 1); EM_REF(\"B\", 2)", &["A", "B"]),
];

#[test]
fn string_literals_reach_the_context_table_on_both_paths() {
    for (source, expected) in LITERAL_CASES {
        let mut tree_ctx = context(5);
        let mut plan_ctx = context(5);

        // The tree path itself is the reference: if it did not build the table,
        // the expectation below would be checking the wrong thing.
        let _ = engine(FormulaExecutionMode::Tree).eval(source, &mut tree_ctx);
        assert_eq!(
            tree_ctx.string_table, *expected,
            "tree path built an unexpected table for `{source}`"
        );

        // The plan path may still fail on a *missing kernel* (see the backlog
        // test below); what it may never do is lose the literal.
        let _ = engine(FormulaExecutionMode::Plan).eval(source, &mut plan_ctx);
        assert_eq!(
            plan_ctx.string_table, *expected,
            "plan path did not publish the literals of `{source}`"
        );
    }
}

#[test]
fn literals_are_offset_by_a_prepopulated_string_table() {
    // The eastern-dialect tests push a name into the table before evaluating, so
    // the tree path's index for the formula's own literal is not zero. The plan
    // binds its own index at compile time, so the dispatcher must add the
    // context's prefix length back.
    let mut tree_ctx = context(5);
    let prefix = tree_ctx.string_table.len();
    tree_ctx.string_table.push("PREEXISTING".to_string());
    let _ = engine(FormulaExecutionMode::Tree).eval("EM_REF(\"IDX\", 1)", &mut tree_ctx);

    let mut plan_ctx = context(5);
    plan_ctx.string_table.push("PREEXISTING".to_string());
    let _ = engine(FormulaExecutionMode::Plan).eval("EM_REF(\"IDX\", 1)", &mut plan_ctx);

    assert_eq!(tree_ctx.string_table, ["PREEXISTING", "IDX"]);
    assert_eq!(plan_ctx.string_table, tree_ctx.string_table);
    assert_eq!(prefix, 0, "context starts with an empty table");
}

/// Run a plan directly and return its first output series.
///
/// This is the production caller contract without the engine in between, which
/// is what lets the offset test below drive a non-zero `string_base`.
fn plan_first_series(source: &str, ctx: &FormulaContext, string_base: usize) -> Vec<f64> {
    let ast = parse_formula(source).expect("parse");
    let plan = FormulaHotPlan::compile(&ast).expect("compile");

    let mut slots: Vec<Option<&[f64]>> = vec![None; plan.hot().input_layout().len()];
    for binding in plan.input_bindings() {
        slots[binding.slot().0] = Some(
            ctx.get_data(binding.name())
                .unwrap_or_else(|| panic!("missing input `{}`", binding.name())),
        );
    }
    let inputs: Vec<&[f64]> = slots
        .into_iter()
        .map(|slot| slot.expect("every input slot is bound in this test"))
        .collect();

    let mut executor = unified_formula_executor(&plan);
    executor.dispatcher_mut().set_string_base(string_base);
    // A literal-only formula binds no input series, so the length has to be
    // supplied explicitly — exactly what the engine does for constant formulas.
    executor
        .execute_range(&inputs, 0..ctx.data_len)
        .expect("plan executes")
        .values
        .into_iter()
        .next()
        .expect("plan produced an output series")
}

#[test]
fn string_literal_kernel_emits_base_plus_local_index() {
    // `EM_REF` cannot run on the plan path yet, so the numeric value of a
    // literal is observed through a formula the dispatcher *can* evaluate: the
    // literal is one argument of a call whose other argument is numeric, and the
    // resulting index is what the arithmetic consumes. `BLOCKAVG("BK") * 0 + 7`
    // would need the missing kernel, so instead the index is read off the
    // dispatcher-facing series of the literal itself.
    let ast = parse_formula("EM_REF(\"IDX\", 1)").expect("parse");
    let plan = FormulaHotPlan::compile(&ast).expect("compile");
    assert_eq!(
        plan.semantic().string_literals_ordered().len(),
        1,
        "the corpus case must carry exactly one literal"
    );

    let ctx = context(4);
    for base in [0usize, 3] {
        // The literal node's own buffer is an internal slot, so the value is
        // checked through a plan whose root *is* the literal: `"X"` alone lowers
        // to `STRING_LITERAL` + `OUTPUT`.
        let series = plan_first_series("\"X\"", &ctx, base);
        assert_eq!(series.len(), 4);
        for value in series {
            assert_eq!(
                value, base as f64,
                "literal index must be the absolute table position (base={base})"
            );
        }
    }
}

/// String-consuming functions the plan dispatcher has no kernel for.
///
/// The literal itself is handled — the text lands in `ctx.string_table` and the
/// index is produced correctly (the tests above prove it) — but these four call
/// targets are absent from the dispatcher, so the arithmetic that consumes the
/// index still cannot run. They are listed rather than left implicit: a stale
/// entry fails the test below, so closing one of them forces this list to
/// shrink.
const STRING_KERNEL_BACKLOG: &[(&str, &str)] = &[
    ("EM_REF(\"IDX\", 1)", "no kernel for CALL:EM_REF"),
    (
        "BLOCKDATA(\"BK\", \"FIELD\")",
        "no kernel for CALL:BLOCKDATA",
    ),
    ("BLOCKINDEX(\"BK\")", "no kernel for CALL:BLOCKINDEX"),
    ("BLOCKAVG(\"BK\")", "no kernel for CALL:BLOCKAVG"),
];

#[test]
fn string_consuming_kernel_backlog_matches_reality() {
    let mut observed = BTreeMap::new();
    for (source, _) in STRING_KERNEL_BACKLOG {
        let mut ctx = context(5);
        let outcome = engine(FormulaExecutionMode::Plan).eval(source, &mut ctx);
        if let Err(error) = outcome {
            let text = error.to_string();
            // A missing kernel is the documented reason. Anything else — a
            // numeric divergence, a lost literal — must fail this test instead
            // of being absorbed into the backlog.
            assert!(
                text.contains("kernel dispatch failed"),
                "`{source}` failed for an unexpected reason: {text}"
            );
            observed.insert(*source, text);
        }
    }

    let mut declared: Vec<&str> = STRING_KERNEL_BACKLOG.iter().map(|(src, _)| *src).collect();
    declared.sort_unstable();
    let actual: Vec<&str> = observed.keys().copied().collect();
    assert_eq!(
        actual, declared,
        "the declared string-kernel backlog no longer matches what the plan path cannot run"
    );
}
