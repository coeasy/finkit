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
//! * the four functions that *consume* that index (`EM_REF`, `BLOCKDATA`,
//!   `BLOCKINDEX`, `BLOCKAVG`) run on the plan path and agree with the tree path
//!   when host data is supplied.
//!
//! The consuming kernels used to be absent, so any formula that read its own
//! literal failed at dispatch. They now resolve the index through the
//! dispatcher's `string_table` and read `ctx.em_data` / `ctx.block_data`, which
//! travel to the plan executor in `HostContext`.

use finkit::formula::{
    parse_formula, unified_formula_executor, BlockData, EmData, FormulaContext, FormulaEngine,
    FormulaExecutionMode, FormulaHotPlan,
};
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

/// The four functions that consume a string literal's index.
///
/// They were the *entire* remaining string-literal gap on the plan path: the
/// literal itself already reached `ctx.string_table` on both paths, but nothing
/// could read it back. The kernels now exist, so these sources must **run** on
/// the plan path (yielding `NaN` without host data) instead of failing with a
/// missing kernel. Keeping the roster here means removing a kernel turns this
/// red rather than silently shrinking a backlog list.
const STRING_CONSUMING_SOURCES: &[&str] = &[
    "EM_REF(\"IDX\", 1)",
    "BLOCKDATA(\"BK\", \"FIELD\")",
    "BLOCKINDEX(\"BK\")",
    "BLOCKAVG(\"BK\")",
];

#[test]
fn string_consuming_functions_run_on_the_plan_path() {
    for source in STRING_CONSUMING_SOURCES {
        let mut ctx = context(5);
        let outcome = engine(FormulaExecutionMode::Plan).eval(source, &mut ctx);
        assert!(
            outcome.is_ok(),
            "`{source}` must run on the plan path now that its kernel exists: {:?}",
            outcome.err()
        );
    }
}

/// A context carrying the host data the consuming kernels read.
fn context_with_host(len: usize) -> FormulaContext {
    let series =
        |base: f64| Array1::from_vec((0..len).map(|i| base + i as f64).collect::<Vec<_>>());

    let mut em = EmData::default();
    em.external_data.insert("IDX".to_string(), series(100.0));

    let mut block = BlockData::default();
    block.index_close.insert("BK".to_string(), series(200.0));
    block.avg_price.insert("BK".to_string(), series(300.0));
    block.pct_change.insert("BK".to_string(), series(0.5));
    block.volume.insert("BK".to_string(), series(1000.0));
    block.amount.insert("BK".to_string(), series(5000.0));
    let mut fields = std::collections::HashMap::new();
    fields.insert("CUSTOM".to_string(), series(7.0));
    block.custom_fields.insert("BK".to_string(), fields);

    context(len).with_em_data(em).with_block_data(block)
}

/// With host data present, the plan kernels must reproduce the tree path
/// *value for value* — not merely both return `NaN`.
///
/// The finite-count assertion is the load-bearing half: an implementation that
/// returned an all-`NaN` series would still agree with itself on both paths, so
/// path equality alone would not notice.
#[test]
fn string_consuming_functions_match_the_tree_path_with_host_data() {
    const CASES: &[&str] = &[
        "EM_REF(\"IDX\", 1)",
        "EM_REF(\"IDX\", 3)",
        "BLOCKINDEX(\"BK\")",
        "BLOCKAVG(\"BK\")",
        "BLOCKDATA(\"BK\", \"INDEX\")",
        "BLOCKDATA(\"BK\", \"VOL\")",
        "BLOCKDATA(\"BK\", \"AMOUNT\")",
        "BLOCKDATA(\"BK\", \"CUSTOM\")",
        "BLOCKAVG(\"BK\") + CLOSE",
    ];

    let finite_count = |series: &[f64]| series.iter().filter(|value| value.is_finite()).count();

    for source in CASES {
        let tree = engine(FormulaExecutionMode::Tree)
            .eval(source, &mut context_with_host(8))
            .unwrap_or_else(|error| panic!("tree failed on `{source}`: {error}"));
        let plan = engine(FormulaExecutionMode::Plan)
            .eval(source, &mut context_with_host(8))
            .unwrap_or_else(|error| panic!("plan failed on `{source}`: {error}"));

        assert_eq!(tree.len(), plan.len(), "length mismatch on `{source}`");
        let tree_values = tree.as_slice().expect("tree output is contiguous");
        let plan_values = plan.as_slice().expect("plan output is contiguous");
        assert!(
            finite_count(tree_values) > 0,
            "`{source}` produced no finite value on the reference path; the case proves nothing"
        );
        assert_eq!(
            finite_count(tree_values),
            finite_count(plan_values),
            "`{source}`: tree and plan disagree on how many finite values they produce"
        );
        for (index, (t, p)) in tree_values.iter().zip(plan_values).enumerate() {
            assert!(
                (t - p).abs() < 1e-12 || (t.is_nan() && p.is_nan()),
                "`{source}` index {index}: tree={t} plan={p}"
            );
        }
    }
}

#[test]
fn string_consuming_functions_preserve_legacy_arity_on_both_backends() {
    for source in ["EM_REF(\"IDX\")", "BLOCKDATA(\"BK\")"] {
        let tree = engine(FormulaExecutionMode::Tree).eval(source, &mut context(5));
        let plan = engine(FormulaExecutionMode::Plan).eval(source, &mut context(5));
        assert!(
            tree.is_err(),
            "tree accepted missing arguments in `{source}`"
        );
        assert!(
            plan.is_err(),
            "plan accepted missing arguments in `{source}`"
        );
    }

    for source in [
        "EM_REF(\"IDX\", 1, CLOSE)",
        "BLOCKDATA(\"BK\", \"CLOSE\", CLOSE)",
        "BLOCKINDEX(\"BK\", CLOSE)",
        "BLOCKAVG(\"BK\", CLOSE)",
    ] {
        let tree = engine(FormulaExecutionMode::Tree).eval(source, &mut context(5));
        let plan = engine(FormulaExecutionMode::Plan).eval(source, &mut context(5));
        assert!(
            tree.is_ok(),
            "legacy tree path rejected trailing args in `{source}`"
        );
        assert!(
            plan.is_ok(),
            "plan path rejected trailing args in `{source}`"
        );
    }
}
